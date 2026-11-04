//! Xray sidecar. sing-box has no client for XHTTP / SplitHTTP, mKCP, TCP with an HTTP header or VLESS encryption,
//! so nodes that need them (`Node::xray`) are carried by an Xray process instead: each gets a local SOCKS5 inbound
//! and sing-box reaches the node through a plain `socks` outbound pointing at it. One Xray process serves every
//! such node of a run (the connection, or one delay-test batch).

use std::{collections::HashMap, net::IpAddr, path::PathBuf, process::Stdio, time::Duration};

use serde_json::{json, Value};
use tauri::{AppHandle, Manager};
use tokio::{
    net::{lookup_host, TcpStream},
    process::{Child, Command},
    time::{sleep, timeout},
};

use crate::{nodes::Node, state::AppState};

pub struct Running {
    child: Child,
    /// Node id -> local SOCKS port.
    pub ports: HashMap<String, u16>,
    /// Addresses Xray dials. TUN would route them back into the tunnel (and into Xray), so they go direct.
    pub server_ips: Vec<String>,
    pub log_path: PathBuf,
}

impl Running {
    /// Last lines Xray wrote (handshake and dial errors), for the Core log.
    pub fn log_tail(&self, lines: usize) -> Vec<String> {
        let text = std::fs::read_to_string(&self.log_path).unwrap_or_default();
        let all: Vec<&str> = text.lines().collect();
        all[all.len().saturating_sub(lines)..].iter().map(|l| format!("xray: {l}")).collect()
    }
}

impl Running {
    pub async fn kill(mut self) {
        let _ = self.child.kill().await;
    }
}

fn binary_name() -> &'static str {
    if cfg!(windows) { "xray.exe" } else { "xray" }
}

/// Same lookup order as sing-box: app data `bin/`, bundled resource, next to the exe, PATH.
#[cfg(not(target_os = "android"))]
async fn binary(app: &AppHandle, state: &AppState) -> Result<PathBuf, String> {
    let mut candidates = vec![state.data_dir.join("bin").join(binary_name())];
    if let Ok(res) = app.path().resource_dir() {
        candidates.push(res.join("binaries").join(binary_name()));
        candidates.push(res.join(binary_name()));
    }
    if let Some(dir) = std::env::current_exe().ok().and_then(|e| e.parent().map(|p| p.to_path_buf())) {
        candidates.push(dir.join("binaries").join(binary_name()));
        candidates.push(dir.join(binary_name()));
    }
    Ok(candidates.into_iter().find(|c| c.exists()).unwrap_or_else(|| binary_name().into()))
}

/// Android only runs executables from the native library directory, where the build ships it as `libxray.so`.
#[cfg(target_os = "android")]
async fn binary(app: &AppHandle, _state: &AppState) -> Result<PathBuf, String> {
    crate::vpn::xray_path(app).await.map(PathBuf::from)
}

/// The sing-box outbound that stands in for `node`: the SOCKS inbound of its Xray process, or the node's own.
pub fn outbound_for(node: &Node, ports: &HashMap<String, u16>) -> Value {
    match ports.get(&node.id) {
        Some(port) => json!({ "type": "socks", "version": "5", "server": "127.0.0.1", "server_port": port }),
        None => node.outbound.clone(),
    }
}

/// Resolves the server once, here: Go's resolver has no system DNS on Android, and the address is needed anyway to
/// keep Xray's own traffic out of the tunnel. TLS names were fixed when the link was parsed, so swapping the
/// address does not change SNI or the Host header.
async fn resolve(outbound: &mut Value) -> Option<String> {
    let account = if outbound["settings"]["vnext"].is_array() { &mut outbound["settings"]["vnext"][0] } else { &mut outbound["settings"]["servers"][0] };
    let host = account["address"].as_str()?.to_string();
    let port = account["port"].as_u64()? as u16;
    let ip = if let Ok(ip) = host.parse::<IpAddr>() {
        ip
    } else {
        let addrs: Vec<_> = timeout(Duration::from_secs(4), lookup_host((host.as_str(), port))).await.ok()?.ok()?.collect();
        addrs.iter().find(|a| a.is_ipv4()).or(addrs.first())?.ip()
    };
    account["address"] = json!(ip.to_string());
    Some(ip.to_string())
}

/// Starts Xray for the nodes that need it. Nodes without an `xray` outbound are ignored; with none left this
/// returns `None`.
pub async fn start(app: &AppHandle, nodes: &[Node]) -> Result<Option<Running>, String> {
    let state = app.state::<AppState>();
    let wanted: Vec<&Node> = nodes.iter().filter(|n| n.xray.is_some()).take(300).collect();
    if wanted.is_empty() {
        return Ok(None);
    }

    let (mut inbounds, mut outbounds, mut rules) = (vec![], vec![], vec![]);
    let (mut ports, mut server_ips) = (HashMap::new(), vec![]);
    let mut used: Vec<u16> = vec![];
    for (i, node) in wanted.iter().enumerate() {
        // The first free port from a per-node starting point; another run's Xray may hold some of them.
        let port = (0u16..500)
            .map(|k| 21080 + i as u16 + k * 7)
            .find(|p| !used.contains(p) && std::net::TcpListener::bind(("127.0.0.1", *p)).is_ok())
            .ok_or("no free local port for xray")?;
        used.push(port);

        let mut outbound = node.xray.clone().unwrap_or_default();
        if let Some(ip) = resolve(&mut outbound).await {
            if !server_ips.contains(&ip) {
                server_ips.push(ip);
            }
        }
        outbound["tag"] = json!(format!("out-{i}"));
        outbounds.push(outbound);
        inbounds.push(json!({
            "tag": format!("in-{i}"), "listen": "127.0.0.1", "port": port, "protocol": "socks",
            "settings": { "auth": "noauth", "udp": true, "ip": "127.0.0.1" },
        }));
        rules.push(json!({ "type": "field", "inboundTag": [format!("in-{i}")], "outboundTag": format!("out-{i}") }));
        ports.insert(node.id.clone(), port);
    }
    outbounds.push(json!({ "protocol": "freedom", "tag": "direct" }));
    let cfg = json!({
        "log": { "loglevel": "warning" },
        "inbounds": inbounds,
        "outbounds": outbounds,
        "routing": { "domainStrategy": "AsIs", "rules": rules },
    });

    let dir = state.data_dir.join("xray");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    // Named after the lowest port so a connection and a delay test running side by side never share files.
    let tag = used.iter().min().copied().unwrap_or(0);
    let cfg_path = dir.join(format!("config-{tag}.json"));
    std::fs::write(&cfg_path, serde_json::to_vec(&cfg).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    let log_path = dir.join(format!("core-{tag}.log"));
    let log = std::fs::File::create(&log_path).map_err(|e| e.to_string())?;
    let log_err = log.try_clone().map_err(|e| e.to_string())?;

    let bin = binary(app, &state).await?;
    let mut cmd = Command::new(&bin);
    cmd.arg("run").arg("-c").arg(&cfg_path).stdout(Stdio::from(log)).stderr(Stdio::from(log_err)).kill_on_drop(true);
    #[cfg(windows)]
    cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    let mut child = cmd.spawn().map_err(|e| format!("cannot start xray ({}): {e}. XHTTP / mKCP nodes need it.", bin.display()))?;

    // Ready once every SOCKS port accepts a connection.
    let probe_port = *used.last().unwrap_or(&0);
    for _ in 0..60 {
        if let Ok(Some(code)) = child.try_wait() {
            let tail = std::fs::read_to_string(&log_path).unwrap_or_default();
            let tail: Vec<&str> = tail.lines().rev().take(4).collect();
            return Err(format!("xray exited ({code}): {}", tail.into_iter().rev().collect::<Vec<_>>().join(" | ")));
        }
        if TcpStream::connect(("127.0.0.1", probe_port)).await.is_ok() {
            let _ = std::fs::remove_file(&cfg_path);
            return Ok(Some(Running { child, ports, server_ips, log_path }));
        }
        sleep(Duration::from_millis(100)).await;
    }
    let _ = child.kill().await;
    Err("xray did not become ready in time".into())
}

/// Kills the Xray process of the current connection, if any.
pub async fn stop(state: &AppState) {
    if let Some(running) = state.xray.lock().await.take() {
        running.kill().await;
    }
}
