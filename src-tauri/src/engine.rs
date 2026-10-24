//! sing-box child-process supervisor, traffic polling and latency probes.

use std::{path::PathBuf, process::Stdio, sync::atomic::Ordering, time::Duration};

use serde::Serialize;
use serde_json::Value;
use tauri::{AppHandle, Emitter, Manager};
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    net::TcpStream,
    process::Command,
    time::{sleep, timeout, Instant},
};

use crate::{
    config, proxy,
    state::{AppState, Engine, Mode},
};

#[derive(Serialize, Clone)]
pub struct CoreStatus {
    pub running: bool,
    pub node_id: Option<String>,
    /// Kill switch is holding traffic after the core died unexpectedly.
    pub blocked: bool,
}

#[derive(Serialize, Clone)]
struct Traffic {
    up: u64,
    down: u64,
    up_total: u64,
    down_total: u64,
}

fn binary_name() -> &'static str {
    if cfg!(windows) { "sing-box.exe" } else { "sing-box" }
}

/// Lookup order: explicit `core_path`, app data `bin/`, bundled resource, next to the exe, PATH.
fn resolve_binary(app: &AppHandle, state: &AppState) -> PathBuf {
    if let Some(p) = state.settings.lock().unwrap().core_path.clone().filter(|p| !p.is_empty()) {
        return p.into();
    }
    let mut candidates = vec![state.data_dir.join("bin").join(binary_name())];
    if let Ok(res) = app.path().resource_dir() {
        candidates.push(res.join("binaries").join(binary_name()));
        candidates.push(res.join(binary_name()));
    }
    if let Some(dir) = std::env::current_exe().ok().and_then(|e| e.parent().map(|p| p.to_path_buf())) {
        candidates.push(dir.join("binaries").join(binary_name()));
        candidates.push(dir.join(binary_name()));
    }
    candidates.into_iter().find(|c| c.exists()).unwrap_or_else(|| binary_name().into())
}

fn free_port(preferred: u16, avoid: Option<u16>) -> u16 {
    let ok = |p: u16| Some(p) != avoid && std::net::TcpListener::bind(("127.0.0.1", p)).is_ok();
    if ok(preferred) {
        return preferred;
    }
    std::net::TcpListener::bind(("127.0.0.1", 0)).and_then(|l| l.local_addr()).map(|a| a.port()).unwrap_or(preferred)
}

pub fn status(app: &AppHandle) -> CoreStatus {
    let state = app.state::<AppState>();
    let node_id = state.active_node.lock().unwrap().clone();
    CoreStatus { running: node_id.is_some(), node_id, blocked: state.blocked.load(Ordering::SeqCst) }
}

fn emit_state(app: &AppHandle) {
    let _ = app.emit("core-state", status(app));
}

pub async fn start(app: &AppHandle) -> Result<CoreStatus, String> {
    let state = app.state::<AppState>();
    let mut child_slot = state.child.lock().await;
    if child_slot.is_some() {
        return Err("core is already running".into());
    }

    let mut settings = state.settings.lock().unwrap().clone();
    // Other proxy apps (Nekoray, v2rayN...) like 2080/7890/9090: fall back to a free port instead of failing.
    settings.mixed_port = free_port(settings.mixed_port, None);
    settings.api_port = free_port(settings.api_port, Some(settings.mixed_port));
    *state.active_ports.lock().unwrap() = (settings.mixed_port, settings.api_port);
    state.blocked.store(false, Ordering::SeqCst);

    if settings.engine == Engine::Xray {
        return Err("xray-core fallback is not implemented yet; switch engine to sing-box".into());
    }
    let node = {
        let nodes = state.nodes.lock().unwrap();
        let id = settings.selected_node.clone().ok_or("no node selected")?;
        nodes.iter().find(|n| n.id == id).cloned().ok_or("selected node no longer exists")?
    };

    let cfg_path = state.data_dir.join("config.json");
    let cfg = serde_json::to_vec_pretty(&config::build(&node, &settings)).map_err(|e| e.to_string())?;
    std::fs::write(&cfg_path, cfg).map_err(|e| e.to_string())?;

    let bin = resolve_binary(app, &state);
    let mut cmd = Command::new(&bin);
    cmd.args(["run", "-c"])
        .arg(&cfg_path)
        .arg("-D")
        .arg(&state.data_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    #[cfg(windows)]
    cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW

    let mut child = cmd.spawn().map_err(|e| {
        format!("cannot start {} ({e}). Put sing-box in {} or set core_path.", bin.display(), state.data_dir.join("bin").display())
    })?;

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<String>();
    for pipe in [
        child.stdout.take().map(|s| Box::new(s) as Box<dyn tokio::io::AsyncRead + Unpin + Send>),
        child.stderr.take().map(|s| Box::new(s) as Box<dyn tokio::io::AsyncRead + Unpin + Send>),
    ]
    .into_iter()
    .flatten()
    {
        let tx = tx.clone();
        let app = app.clone();
        tauri::async_runtime::spawn(async move {
            let mut lines = BufReader::new(pipe).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                let _ = app.emit("core-log", &line);
                let _ = tx.send(line);
            }
        });
    }
    drop(tx);

    // Wait until the Clash API answers, or the process dies.
    let api = format!("http://127.0.0.1:{}/version", settings.api_port);
    let client = reqwest::Client::builder().no_proxy().build().unwrap_or_default();
    let mut ready = false;
    let mut last_lines: Vec<String> = vec![];
    for _ in 0..40 {
        while let Ok(l) = rx.try_recv() {
            last_lines.push(l);
        }
        if let Ok(Some(code)) = child.try_wait() {
            let tail = last_lines.iter().rev().take(4).rev().cloned().collect::<Vec<_>>().join("\n");
            return Err(format!("core exited ({code}):\n{tail}"));
        }
        if client.get(&api).timeout(Duration::from_millis(300)).send().await.is_ok() {
            ready = true;
            break;
        }
        sleep(Duration::from_millis(150)).await;
    }
    if !ready {
        let _ = child.kill().await;
        return Err("core did not become ready in time".into());
    }

    if settings.mode == Mode::SystemProxy {
        if let Err(e) = proxy::set(&format!("127.0.0.1:{}", settings.mixed_port)) {
            let _ = child.kill().await;
            return Err(format!("failed to set system proxy: {e}"));
        }
    }

    *child_slot = Some(child);
    *state.active_node.lock().unwrap() = Some(node.id.clone());
    drop(child_slot);

    let generation = state.generation.fetch_add(1, Ordering::SeqCst) + 1;
    tauri::async_runtime::spawn(watchdog(app.clone(), generation, node.id.clone()));
    let task = tauri::async_runtime::spawn(traffic_loop(app.clone(), settings.api_port));
    if let Some(old) = state.traffic_task.lock().unwrap().replace(task) {
        old.abort();
    }
    emit_state(app);
    Ok(status(app))
}

pub async fn stop(app: &AppHandle) -> Result<CoreStatus, String> {
    let state = app.state::<AppState>();
    state.generation.fetch_add(1, Ordering::SeqCst); // retires the watchdog
    state.blocked.store(false, Ordering::SeqCst);
    if let Some(t) = state.traffic_task.lock().unwrap().take() {
        t.abort();
    }
    let _ = proxy::clear(); // always restore, even if we were not the ones to set it
    if let Some(mut child) = state.child.lock().await.take() {
        let _ = child.kill().await;
    }
    *state.active_node.lock().unwrap() = None;
    emit_state(app);
    Ok(status(app))
}

async fn traffic_loop(app: AppHandle, api_port: u16) {
    let client = reqwest::Client::builder().no_proxy().build().unwrap_or_default();
    let url = format!("http://127.0.0.1:{api_port}/connections");
    let mut prev: Option<(u64, u64, Instant)> = None;
    loop {
        sleep(Duration::from_secs(1)).await;
        let Ok(resp) = client.get(&url).timeout(Duration::from_secs(2)).send().await else { continue };
        let Ok(v) = resp.json::<Value>().await else { continue };
        let (up_total, down_total) = (v["uploadTotal"].as_u64().unwrap_or(0), v["downloadTotal"].as_u64().unwrap_or(0));
        let now = Instant::now();
        let (up, down) = match prev {
            Some((pu, pd, t)) => {
                let dt = now.duration_since(t).as_secs_f64().max(0.001);
                ((up_total.saturating_sub(pu) as f64 / dt) as u64, (down_total.saturating_sub(pd) as f64 / dt) as u64)
            }
            None => (0, 0),
        };
        prev = Some((up_total, down_total, now));
        let _ = app.emit("traffic", Traffic { up, down, up_total, down_total });
    }
}

/// TCP connect time to the node's server, in ms.
pub async fn tcp_ping(server: &str, port: u16) -> Option<u32> {
    let t = Instant::now();
    timeout(Duration::from_secs(3), TcpStream::connect((server, port)))
        .await
        .ok()?
        .ok()?;
    Some(t.elapsed().as_millis() as u32)
}

/// Real HTTP round-trip through the running core's local mixed inbound.
pub async fn http_ping(port: u16, url: &str) -> Result<u32, String> {
    let client = reqwest::Client::builder()
        .proxy(reqwest::Proxy::all(format!("http://127.0.0.1:{port}")).map_err(|e| e.to_string())?)
        .timeout(Duration::from_secs(8))
        .build()
        .map_err(|e| e.to_string())?;
    let t = Instant::now();
    client.get(url).send().await.map_err(|e| e.to_string())?;
    Ok(t.elapsed().as_millis() as u32)
}

/// Supervises a running core: detects an unexpected exit (kill switch) and a dead node (auto fallback).
async fn watchdog(app: AppHandle, generation: u64, node_id: String) {
    let state = app.state::<AppState>();
    let alive = || state.generation.load(Ordering::SeqCst) == generation;
    let mut failures = 0u32;
    loop {
        sleep(Duration::from_secs(8)).await;
        if !alive() {
            return;
        }
        let (mode, kill_switch, fallback, ping_url) = {
            let s = state.settings.lock().unwrap();
            (s.mode, s.kill_switch, s.auto_fallback, s.ping_url.clone())
        };

        let exited = {
            let mut slot = state.child.lock().await;
            slot.as_mut().is_some_and(|c| matches!(c.try_wait(), Ok(Some(_)))) && slot.take().is_some()
        };
        if exited {
            if let Some(t) = state.traffic_task.lock().unwrap().take() {
                t.abort();
            }
            *state.active_node.lock().unwrap() = None;
            if kill_switch && mode == Mode::SystemProxy {
                // Leave the system proxy pointing at the dead port: apps fail closed instead of leaking.
                state.blocked.store(true, Ordering::SeqCst);
            } else {
                let _ = proxy::clear();
            }
            state.generation.fetch_add(1, Ordering::SeqCst);
            emit_state(&app);
            let _ = app.emit("core-log", "core exited unexpectedly");
            return;
        }

        let port = state.active_ports.lock().unwrap().0;
        match http_ping(port, &ping_url).await {
            Ok(_) => failures = 0,
            Err(_) => failures += 1,
        }
        if failures >= 3 && fallback {
            if let Some(next) = pick_alternative(&app, &node_id).await {
                state.settings.lock().unwrap().selected_node = Some(next.clone());
                let _ = state.save_settings();
                let name = state.nodes.lock().unwrap().iter().find(|n| n.id == next).map(|n| n.name.clone()).unwrap_or_default();
                let _ = app.emit("core-log", format!("node unreachable, switching to {name}"));
                let _ = app.emit("fallback", name);
                // Restarted from lib.rs (a listener) to avoid a start -> watchdog -> start async cycle.
                let _ = app.emit("restart-core", ());
                return;
            }
            failures = 0;
        }
    }
}

/// Lowest-latency node other than `current` that answers a TCP connect.
async fn pick_alternative(app: &AppHandle, current: &str) -> Option<String> {
    let state = app.state::<AppState>();
    let targets: Vec<(String, String, u16)> = state
        .nodes
        .lock()
        .unwrap()
        .iter()
        .filter(|n| n.id != current)
        .map(|n| (n.id.clone(), n.server.clone(), n.port))
        .collect();
    let handles: Vec<_> = targets
        .into_iter()
        .map(|(id, server, port)| tauri::async_runtime::spawn(async move { (id, tcp_ping(&server, port).await) }))
        .collect();
    let mut best: Option<(String, u32)> = None;
    for h in handles {
        if let Ok((id, Some(ms))) = h.await {
            if best.as_ref().is_none_or(|(_, b)| ms < *b) {
                best = Some((id, ms));
            }
        }
    }
    best.map(|(id, _)| id)
}

#[derive(Serialize, Clone)]
pub struct ExitInfo {
    pub ip: String,
    pub country: String,
    pub country_code: String,
}

/// Public IP and country as seen from outside, through the running core.
pub async fn exit_info(port: u16) -> Result<ExitInfo, String> {
    let client = reqwest::Client::builder()
        .proxy(reqwest::Proxy::all(format!("http://127.0.0.1:{port}")).map_err(|e| e.to_string())?)
        .timeout(Duration::from_secs(8))
        .build()
        .map_err(|e| e.to_string())?;
    let v: Value = client
        .get("http://ip-api.com/json/?fields=status,query,country,countryCode")
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;
    if v["status"] != "success" {
        return Err("exit IP lookup failed".into());
    }
    let s = |k: &str| v[k].as_str().unwrap_or_default().to_string();
    Ok(ExitInfo { ip: s("query"), country: s("country"), country_code: s("countryCode") })
}
