//! `#[tauri::command]` surface exposed to the frontend.

use tauri::{AppHandle, State};

use crate::{
    autostart,
    engine::{self, CoreStatus, ExitInfo},
    netcheck::{self, Diagnostics, SpeedResult},
    nodes::{self, Node},
    probe,
    state::{AppState, Settings},
    subs::{self, ImportResult, Subscription},
    updater::{self, UpdateInfo},
};

/// Selects the first node when nothing is selected yet, so a fresh import is immediately connectable.
pub fn ensure_selection(state: &AppState) -> Result<(), String> {
    let first = state.nodes.lock().unwrap().first().map(|n| n.id.clone());
    let changed = {
        let mut s = state.settings.lock().unwrap();
        let valid = s.selected_node.as_ref().is_some_and(|id| state.nodes.lock().unwrap().iter().any(|n| &n.id == id));
        if !valid && first.is_some() {
            s.selected_node = first;
            true
        } else {
            false
        }
    };
    if changed { state.save_settings() } else { Ok(()) }
}

#[tauri::command]
pub fn list_nodes(state: State<'_, AppState>) -> Vec<Node> {
    state.nodes.lock().unwrap().clone()
}

#[tauri::command]
pub fn import_text(text: String, state: State<'_, AppState>) -> Result<ImportResult, String> {
    let (parsed, errors) = nodes::parse_text(&text);
    let added = parsed.len();
    state.nodes.lock().unwrap().extend(parsed);
    state.save_nodes()?;
    ensure_selection(&state)?;
    Ok(ImportResult { added, errors })
}

#[tauri::command]
pub fn list_subscriptions(state: State<'_, AppState>) -> Vec<Subscription> {
    state.subs.lock().unwrap().clone()
}

/// Creates a named subscription and downloads it. On failure nothing is kept.
#[tauri::command]
pub async fn add_subscription(name: String, url: String, app: AppHandle, state: State<'_, AppState>) -> Result<ImportResult, String> {
    let url = url.trim().to_string();
    if !url.to_ascii_lowercase().starts_with("http") {
        return Err("subscription URL must start with http:// or https://".into());
    }
    if state.subs.lock().unwrap().iter().any(|s| s.url == url) {
        return Err("this subscription is already imported".into());
    }
    let name = if name.trim().is_empty() {
        url::Url::parse(&url).ok().and_then(|u| u.host_str().map(str::to_string)).unwrap_or_else(|| "Subscription".into())
    } else {
        name.trim().to_string()
    };
    let sub = Subscription::new(name, url);
    let id = sub.id.clone();
    state.subs.lock().unwrap().push(sub);
    match subs::refresh(&app, &id).await {
        Ok(r) => Ok(r),
        Err(e) => {
            state.subs.lock().unwrap().retain(|s| s.id != id);
            Err(e)
        }
    }
}

#[tauri::command]
pub async fn refresh_subscription(id: String, app: AppHandle) -> Result<ImportResult, String> {
    subs::refresh(&app, &id).await
}

#[tauri::command]
pub fn rename_subscription(id: String, name: String, state: State<'_, AppState>) -> Result<(), String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("name cannot be empty".into());
    }
    if let Some(s) = state.subs.lock().unwrap().iter_mut().find(|s| s.id == id) {
        s.name = name.to_string();
    }
    state.save_subs()
}

/// Removes the subscription together with its nodes.
#[tauri::command]
pub fn remove_subscription(id: String, state: State<'_, AppState>) -> Result<(), String> {
    state.subs.lock().unwrap().retain(|s| s.id != id);
    state.nodes.lock().unwrap().retain(|n| n.subscription.as_deref() != Some(id.as_str()));
    state.save_subs()?;
    state.save_nodes()?;
    {
        let nodes = state.nodes.lock().unwrap();
        let mut s = state.settings.lock().unwrap();
        if s.selected_node.as_ref().is_some_and(|sel| !nodes.iter().any(|n| &n.id == sel)) {
            s.selected_node = None;
        }
    }
    ensure_selection(&state)?;
    state.save_settings()
}

#[tauri::command]
pub fn remove_node(id: String, state: State<'_, AppState>) -> Result<(), String> {
    state.nodes.lock().unwrap().retain(|n| n.id != id);
    {
        let mut s = state.settings.lock().unwrap();
        if s.selected_node.as_deref() == Some(id.as_str()) {
            s.selected_node = None;
        }
    }
    state.save_nodes()?;
    state.save_settings()
}

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Settings {
    state.settings.lock().unwrap().clone()
}

#[tauri::command]
pub fn set_settings(settings: Settings, state: State<'_, AppState>) -> Result<(), String> {
    let autostart_changed = state.settings.lock().unwrap().autostart != settings.autostart;
    if autostart_changed {
        autostart::set(settings.autostart)?;
    }
    *state.settings.lock().unwrap() = settings;
    state.save_settings()
}

#[tauri::command]
pub fn select_node(id: String, state: State<'_, AppState>) -> Result<(), String> {
    state.settings.lock().unwrap().selected_node = Some(id);
    state.save_settings()
}

#[tauri::command]
pub async fn connect(app: AppHandle) -> Result<CoreStatus, String> {
    engine::start(&app).await
}

#[tauri::command]
pub async fn disconnect(app: AppHandle) -> Result<CoreStatus, String> {
    engine::stop(&app).await
}

#[tauri::command]
pub fn core_status(app: AppHandle) -> CoreStatus {
    engine::status(&app)
}

fn store_latency(state: &AppState, id: &str, ms: Option<u32>) {
    if let Some(n) = state.nodes.lock().unwrap().iter_mut().find(|n| n.id == id) {
        n.latency = ms;
    }
}

/// TCP connect latency of one node (works while disconnected).
#[tauri::command]
pub async fn tcp_ping(id: String, state: State<'_, AppState>) -> Result<Option<u32>, String> {
    let (server, port) = state
        .nodes
        .lock()
        .unwrap()
        .iter()
        .find(|n| n.id == id)
        .map(|n| (n.server.clone(), n.port))
        .ok_or("unknown node")?;
    let ms = engine::tcp_ping(&server, port).await;
    store_latency(&state, &id, ms);
    Ok(ms)
}

#[tauri::command]
pub async fn tcp_ping_all(state: State<'_, AppState>) -> Result<(), String> {
    let targets: Vec<(String, String, u16)> = state
        .nodes
        .lock()
        .unwrap()
        .iter()
        .map(|n| (n.id.clone(), n.server.clone(), n.port))
        .collect();
    let results = futures_join(targets).await;
    for (id, ms) in results {
        store_latency(&state, &id, ms);
    }
    state.save_nodes()
}

async fn futures_join(targets: Vec<(String, String, u16)>) -> Vec<(String, Option<u32>)> {
    let handles: Vec<_> = targets
        .into_iter()
        .map(|(id, server, port)| tauri::async_runtime::spawn(async move { (id, engine::tcp_ping(&server, port).await) }))
        .collect();
    let mut out = vec![];
    for h in handles {
        if let Ok(r) = h.await {
            out.push(r);
        }
    }
    out
}

/// Real HTTP round-trip through the active tunnel; requires being connected.
#[tauri::command]
pub async fn http_ping(state: State<'_, AppState>) -> Result<u32, String> {
    if state.active_node.lock().unwrap().is_none() {
        return Err("connect first to measure real HTTP latency".into());
    }
    let (port, url) = {
        let ping_url = state.settings.lock().unwrap().ping_url.clone();
        (state.active_ports.lock().unwrap().0, ping_url)
    };
    let ms = engine::http_ping(port, &url).await?;
    if let Some(id) = state.active_node.lock().unwrap().clone() {
        if let Some(n) = state.nodes.lock().unwrap().iter_mut().find(|n| n.id == id) {
            n.delay = Some(ms as i32);
        }
    }
    Ok(ms)
}

/// TCP-pings every node, selects the fastest reachable one and returns its id.
#[tauri::command]
pub async fn pick_fastest(state: State<'_, AppState>) -> Result<Option<String>, String> {
    tcp_ping_all(state.clone()).await?;
    let best = state
        .nodes
        .lock()
        .unwrap()
        .iter()
        .filter_map(|n| n.latency.map(|l| (l, n.id.clone())))
        .min()
        .map(|(_, id)| id);
    if let Some(id) = &best {
        state.settings.lock().unwrap().selected_node = Some(id.clone());
        state.save_settings()?;
    }
    Ok(best)
}

/// Exit IP / country through the active tunnel.
#[tauri::command]
pub async fn exit_info(state: State<'_, AppState>) -> Result<ExitInfo, String> {
    if state.active_node.lock().unwrap().is_none() {
        return Err("not connected".into());
    }
    let port = state.active_ports.lock().unwrap().0;
    engine::exit_info(port).await
}

/// Real HTTP delay through each node (separate headless core), streamed as `node-delay` events.
#[tauri::command]
pub async fn real_delay(ids: Vec<String>, app: AppHandle) -> Result<(), String> {
    probe::real_delay(&app, ids).await
}

#[tauri::command]
pub async fn speed_test(app: AppHandle) -> Result<SpeedResult, String> {
    netcheck::speed_test(&app).await
}

#[tauri::command]
pub async fn diagnostics(app: AppHandle) -> Result<Diagnostics, String> {
    netcheck::diagnostics(&app).await
}

/// Back to factory settings. Nodes, subscriptions and the selected node are kept.
#[tauri::command]
pub fn reset_settings(state: State<'_, AppState>) -> Result<Settings, String> {
    let was_autostart = state.settings.lock().unwrap().autostart;
    if was_autostart {
        autostart::set(false)?;
    }
    let fresh = {
        let mut s = state.settings.lock().unwrap();
        let selected = s.selected_node.take();
        *s = Settings { selected_node: selected, ..Settings::default() };
        s.clone()
    };
    state.save_settings()?;
    Ok(fresh)
}

#[derive(serde::Serialize)]
pub struct RunningApp {
    pub name: String,
    pub title: String,
}

/// Programs that currently have a window (so the list is apps, not hundreds of services).
#[tauri::command]
pub async fn list_running_apps() -> Result<Vec<RunningApp>, String> {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let out = tokio::task::spawn_blocking(|| {
            std::process::Command::new("tasklist").args(["/v", "/fo", "csv", "/nh"]).creation_flags(0x0800_0000).output()
        })
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;
        let text = String::from_utf8_lossy(&out.stdout);
        let mut apps: Vec<RunningApp> = vec![];
        for line in text.lines() {
            // "Image","PID","Session","Session#","Mem","Status","User","CPU","Window title"
            let cols: Vec<&str> = line.trim().trim_matches('"').split("\",\"").collect();
            let (Some(name), Some(title)) = (cols.first(), cols.last()) else { continue };
            let lower = name.to_ascii_lowercase();
            if cols.len() < 9 || *title == "N/A" || title.is_empty() || lower == "vanish.exe" || lower == "sing-box.exe" {
                continue;
            }
            if !apps.iter().any(|a| a.name.eq_ignore_ascii_case(name)) {
                apps.push(RunningApp { name: name.to_string(), title: title.to_string() });
            }
        }
        apps.sort_by_key(|a| a.name.to_ascii_lowercase());
        Ok(apps)
    }
    #[cfg(not(windows))]
    Err("listing running apps is only implemented for Windows".into())
}

/// Native file dialog; returns the chosen program's file name (what sing-box matches on).
#[tauri::command]
pub async fn pick_app_exe(app: AppHandle) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;
    let picked = tokio::task::spawn_blocking(move || {
        let mut d = app.dialog().file().set_title("Choose an application");
        if cfg!(windows) {
            d = d.add_filter("Applications", &["exe"]);
        }
        d.blocking_pick_file()
    })
    .await
    .map_err(|e| e.to_string())?;
    Ok(picked
        .and_then(|f| f.into_path().ok())
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned())))
}

#[tauri::command]
pub fn app_version(app: AppHandle) -> String {
    app.package_info().version.to_string()
}

#[tauri::command]
pub async fn check_update(app: AppHandle) -> Result<Option<UpdateInfo>, String> {
    updater::check(&app).await
}

#[tauri::command]
pub async fn install_update(app: AppHandle) -> Result<(), String> {
    updater::install(&app).await
}
