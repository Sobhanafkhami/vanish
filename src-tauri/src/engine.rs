//! sing-box child-process supervisor, traffic polling and latency probes.

use std::{path::PathBuf, process::Stdio, time::Duration};

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

fn resolve_binary(state: &AppState) -> PathBuf {
    if let Some(p) = state.settings.lock().unwrap().core_path.clone().filter(|p| !p.is_empty()) {
        return p.into();
    }
    let local = state.data_dir.join("bin").join(binary_name());
    if local.exists() { local } else { binary_name().into() } // fall back to PATH
}

pub fn status(app: &AppHandle) -> CoreStatus {
    let state = app.state::<AppState>();
    let node_id = state.active_node.lock().unwrap().clone();
    CoreStatus { running: node_id.is_some(), node_id }
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

    let settings = state.settings.lock().unwrap().clone();
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

    let bin = resolve_binary(&state);
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

    let task = tauri::async_runtime::spawn(traffic_loop(app.clone(), settings.api_port));
    if let Some(old) = state.traffic_task.lock().unwrap().replace(task) {
        old.abort();
    }
    emit_state(app);
    Ok(status(app))
}

pub async fn stop(app: &AppHandle) -> Result<CoreStatus, String> {
    let state = app.state::<AppState>();
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
