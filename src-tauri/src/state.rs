use std::{
    fs,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, AtomicU64},
        Mutex,
    },
};

use serde::{Deserialize, Serialize};
use tokio::process::Child;

use crate::{nodes::Node, subs::Subscription};

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    SystemProxy,
    Tun,
    /// Only expose the local mixed inbound, don't touch the OS.
    Manual,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "snake_case")]
pub enum Engine {
    SingBox,
    Xray,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "snake_case")]
pub enum RouteMode {
    /// Everything through the proxy (except private networks).
    Global,
    /// `.ir` and the user's direct lists bypass the proxy, the rest goes through it.
    Rule,
    /// Everything direct.
    Direct,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Settings {
    pub mode: Mode,
    pub engine: Engine,
    pub mixed_port: u16,
    pub api_port: u16,
    pub core_path: Option<String>,
    pub selected_node: Option<String>,
    pub ping_url: String,
    pub route_mode: RouteMode,
    /// Domain suffixes that always go direct (any route mode).
    pub direct_domains: Vec<String>,
    /// Process names (e.g. `steam.exe`) that always go direct.
    pub direct_apps: Vec<String>,
    /// DoH resolver used through the tunnel.
    pub dns_server: String,
    pub autostart: bool,
    pub connect_on_launch: bool,
    /// Switch to the next-best node when the current one stops answering.
    pub auto_fallback: bool,
    /// If the core dies unexpectedly, keep the system proxy pointing at it so traffic is blocked, not leaked.
    pub kill_switch: bool,
    /// Hours between automatic subscription refreshes; 0 disables.
    pub sub_refresh_hours: u32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            mode: Mode::SystemProxy,
            engine: Engine::SingBox,
            mixed_port: 17890,
            api_port: 19090,
            core_path: None,
            selected_node: None,
            ping_url: "https://www.gstatic.com/generate_204".into(),
            route_mode: RouteMode::Global,
            direct_domains: vec![],
            direct_apps: vec![],
            dns_server: "1.1.1.1".into(),
            autostart: false,
            connect_on_launch: false,
            auto_fallback: false,
            kill_switch: false,
            sub_refresh_hours: 12,
        }
    }
}

pub struct AppState {
    pub data_dir: PathBuf,
    pub nodes: Mutex<Vec<Node>>,
    pub subs: Mutex<Vec<Subscription>>,
    pub settings: Mutex<Settings>,
    pub child: tokio::sync::Mutex<Option<Child>>,
    pub traffic_task: Mutex<Option<tauri::async_runtime::JoinHandle<()>>>,
    pub active_node: Mutex<Option<String>>,
    /// (mixed, api) ports of the running core; may differ from settings if those were busy.
    pub active_ports: Mutex<(u16, u16)>,
    /// Bumped on every start/stop so a stale watchdog knows to exit.
    pub generation: AtomicU64,
    /// True while the kill switch is holding traffic after an unexpected core exit.
    pub blocked: AtomicBool,
}

/// Missing file -> default. Unreadable/corrupt file -> default, but the original is kept as
/// `<name>.corrupt` so a bad parse can never be silently overwritten by the next save.
fn read_json<T: for<'de> Deserialize<'de> + Default>(path: &Path) -> T {
    let Ok(bytes) = fs::read(path) else { return T::default() };
    match serde_json::from_slice(&bytes) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("failed to parse {}: {e}", path.display());
            let _ = fs::rename(path, path.with_extension("json.corrupt"));
            T::default()
        }
    }
}

/// Write-then-rename so a crash mid-write cannot truncate the existing file.
fn write_atomic(path: &Path, data: &[u8]) -> Result<(), String> {
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, data).map_err(|e| e.to_string())?;
    fs::rename(&tmp, path).map_err(|e| e.to_string())
}

/// Early builds stored the subscription URL on each node. Turn those into real subscriptions.
fn migrate_legacy_subs(nodes: &mut [Node], subs: &mut Vec<Subscription>) -> bool {
    let mut changed = false;
    for n in nodes.iter_mut() {
        let Some(url) = n.subscription.clone().filter(|s| s.starts_with("http")) else { continue };
        let id = match subs.iter().find(|s| s.url == url) {
            Some(s) => s.id.clone(),
            None => {
                let name = url::Url::parse(&url).ok().and_then(|u| u.host_str().map(str::to_string)).unwrap_or_else(|| "Subscription".into());
                let sub = Subscription::new(name, url);
                let id = sub.id.clone();
                subs.push(sub);
                id
            }
        };
        n.subscription = Some(id);
        changed = true;
    }
    changed
}

/// Older builds could give several nodes of one subscription the same id (they matched on server:port only),
/// which made them select together. Give every repeat a fresh id; the first keeps its own so a selection survives.
fn dedupe_ids(nodes: &mut [Node]) -> bool {
    let mut seen = std::collections::HashSet::new();
    let mut changed = false;
    for n in nodes.iter_mut() {
        if !seen.insert(n.id.clone()) {
            n.id = uuid::Uuid::new_v4().to_string();
            seen.insert(n.id.clone());
            changed = true;
        }
    }
    changed
}

impl AppState {
    pub fn load(data_dir: PathBuf) -> Self {
        let mut nodes = read_json::<Vec<Node>>(&data_dir.join("nodes.json"));
        let mut subs = read_json::<Vec<Subscription>>(&data_dir.join("subscriptions.json"));
        let settings = read_json::<Settings>(&data_dir.join("settings.json"));
        let migrated = migrate_legacy_subs(&mut nodes, &mut subs) | dedupe_ids(&mut nodes);
        let state = Self {
            data_dir,
            nodes: Mutex::new(nodes),
            subs: Mutex::new(subs),
            settings: Mutex::new(settings),
            child: tokio::sync::Mutex::new(None),
            traffic_task: Mutex::new(None),
            active_node: Mutex::new(None),
            active_ports: Mutex::new((0, 0)),
            generation: AtomicU64::new(0),
            blocked: AtomicBool::new(false),
        };
        if migrated {
            let _ = state.save_nodes();
            let _ = state.save_subs();
        }
        state
    }

    pub fn save_nodes(&self) -> Result<(), String> {
        let json = serde_json::to_vec_pretty(&*self.nodes.lock().unwrap()).map_err(|e| e.to_string())?;
        write_atomic(&self.data_dir.join("nodes.json"), &json)
    }

    pub fn save_subs(&self) -> Result<(), String> {
        let json = serde_json::to_vec_pretty(&*self.subs.lock().unwrap()).map_err(|e| e.to_string())?;
        write_atomic(&self.data_dir.join("subscriptions.json"), &json)
    }

    pub fn save_settings(&self) -> Result<(), String> {
        let json = serde_json::to_vec_pretty(&*self.settings.lock().unwrap()).map_err(|e| e.to_string())?;
        write_atomic(&self.data_dir.join("settings.json"), &json)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duplicate_node_ids_are_repaired() {
        let node = |id: &str| Node { id: id.into(), ..Node::default() };
        let mut nodes = vec![node("a"), node("a"), node("a"), node("b")];
        assert!(dedupe_ids(&mut nodes));
        let ids: std::collections::HashSet<_> = nodes.iter().map(|n| n.id.clone()).collect();
        assert_eq!(ids.len(), 4);
        assert_eq!(nodes[0].id, "a"); // the first keeps its id
        assert!(!dedupe_ids(&mut nodes));
    }

    #[test]
    fn legacy_url_subscriptions_become_named_subs() {
        let node = |url: &str| Node { subscription: Some(url.into()), ..Node::default() };
        let mut nodes = vec![node("https://a.example/sub"), node("https://a.example/sub"), Node::default()];
        let mut subs = vec![];
        assert!(migrate_legacy_subs(&mut nodes, &mut subs));
        assert_eq!(subs.len(), 1);
        assert_eq!(subs[0].name, "a.example");
        assert_eq!(nodes[0].subscription.as_deref(), Some(subs[0].id.as_str()));
        assert_eq!(nodes[1].subscription, nodes[0].subscription);
        assert_eq!(nodes[2].subscription, None);
        // already migrated -> no-op
        assert!(!migrate_legacy_subs(&mut nodes, &mut subs));
    }
}
