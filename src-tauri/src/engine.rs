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
pub fn resolve_binary(app: &AppHandle, state: &AppState) -> PathBuf {
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

pub fn free_port(preferred: u16, avoid: Option<u16>) -> u16 {
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
    if settings.game_mode {
        // Games talk UDP and do not follow system proxy settings, so only TUN can carry them.
        settings.mode = Mode::Tun;
    }
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
    let _ = state.save_nodes(); // persist the usage counters
    emit_state(app);
    Ok(status(app))
}

async fn traffic_loop(app: AppHandle, api_port: u16) {
    let client = reqwest::Client::builder().no_proxy().build().unwrap_or_default();
    let url = format!("http://127.0.0.1:{api_port}/connections");
    let mut prev: Option<(u64, u64, Instant)> = None;
    // Totals already credited to the active node (the core's counters restart with every start).
    let mut credited = (0u64, 0u64);
    let mut ticks = 0u32;
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

        let state = app.state::<AppState>();
        let active = state.active_node.lock().unwrap().clone();
        if let Some(id) = active {
            let (du, dd) = (up_total.saturating_sub(credited.0), down_total.saturating_sub(credited.1));
            if let Some(n) = state.nodes.lock().unwrap().iter_mut().find(|n| n.id == id) {
                n.up_total += du;
                n.down_total += dd;
            }
            credited = (up_total, down_total);
            ticks += 1;
            if ticks % 15 == 0 {
                let _ = state.save_nodes();
                let _ = app.emit("usage-changed", ());
            }
        }
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

fn proxied_client(port: u16) -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .proxy(reqwest::Proxy::all(format!("http://127.0.0.1:{port}")).map_err(|e| e.to_string())?)
        .timeout(Duration::from_secs(8))
        .tcp_nodelay(true)
        .build()
        .map_err(|e| e.to_string())
}

async fn timed_get(client: &reqwest::Client, url: &str) -> Result<u32, String> {
    let t = Instant::now();
    client.get(url).send().await.map_err(|e| e.to_string())?;
    Ok(t.elapsed().as_millis() as u32)
}

/// Real HTTP round-trip through the running core's local mixed inbound.
///
/// The first request pays for everything that happens once per connection: dialing the server, the proxy
/// handshake and (for HTTPS) TLS. That is 3-4 round trips and is what made Vanish read far higher than other
/// clients. A second request on the same connection is the round trip a game or a page load actually sees,
/// so that is the number reported (the first one is the fallback if the connection is not reused).
pub async fn http_ping(port: u16, url: &str) -> Result<u32, String> {
    let client = proxied_client(port)?;
    let cold = timed_get(&client, url).await?;
    Ok(timed_get(&client, url).await.map_or(cold, |warm| warm.min(cold)))
}

/// Supervises a running core: detects an unexpected exit (kill switch) and a dead node (auto fallback).
async fn watchdog(app: AppHandle, generation: u64, node_id: String) {
    let state = app.state::<AppState>();
    let alive = || state.generation.load(Ordering::SeqCst) == generation;
    let mut failures = 0u32;
    let game = state.settings.lock().unwrap().game_mode;
    // Game mode keeps one connection open and probes it often: it doubles as a keep-alive for the tunnel and
    // feeds the live ping / jitter / loss readout, and a dead path is noticed in about ten seconds instead of 24.
    let mut monitor = GameMonitor::default();
    let mut game_client: Option<(u16, reqwest::Client)> = None;
    loop {
        sleep(Duration::from_secs(if game { 2 } else { 8 })).await;
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
        if game {
            if game_client.as_ref().is_none_or(|(p, _)| *p != port) {
                game_client = proxied_client(port).ok().map(|c| (port, c));
            }
            let sample = match &game_client {
                Some((_, c)) => match timeout(Duration::from_secs(4), timed_get(c, &ping_url)).await {
                    Ok(Ok(ms)) => Some(ms),
                    _ => None,
                },
                None => None,
            };
            failures = if sample.is_some() { 0 } else { failures + 1 };
            let _ = app.emit("game-stats", monitor.record(sample));
        } else {
            match http_ping(port, &ping_url).await {
                Ok(_) => failures = 0,
                Err(_) => failures += 1,
            }
        }
        // Three misses in a row: 24 s normally, 6 s in game mode (where fallback is always on).
        if failures >= 3 && (fallback || game) {
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

#[derive(Serialize, Clone)]
pub struct GameStats {
    /// Smoothed round trip in ms (last few samples), None while nothing has answered.
    pub ping: Option<u32>,
    /// Mean change between consecutive samples, ms.
    pub jitter: u32,
    /// Share of the recent window that got no answer, 0-100.
    pub loss: u32,
    /// Raw recent samples for the sparkline; -1 marks a lost probe.
    pub samples: Vec<i32>,
}

#[derive(Default)]
struct GameMonitor {
    window: std::collections::VecDeque<Option<u32>>,
}

impl GameMonitor {
    const WINDOW: usize = 30;

    fn record(&mut self, sample: Option<u32>) -> GameStats {
        if self.window.len() == Self::WINDOW {
            self.window.pop_front();
        }
        self.window.push_back(sample);
        let ok: Vec<u32> = self.window.iter().flatten().copied().collect();
        let recent: Vec<u32> = self.window.iter().rev().take(5).flatten().copied().collect();
        let ping = (!recent.is_empty()).then(|| recent.iter().sum::<u32>() / recent.len() as u32);
        let jitter = if ok.len() > 1 { ok.windows(2).map(|w| w[0].abs_diff(w[1])).sum::<u32>() / (ok.len() as u32 - 1) } else { 0 };
        let loss = (self.window.iter().filter(|s| s.is_none()).count() * 100 / self.window.len()) as u32;
        GameStats { ping, jitter, loss, samples: self.window.iter().map(|s| s.map_or(-1, |v| v as i32)).collect() }
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
    pub region: String,
    pub city: String,
    pub zip: String,
    pub host: String,
    pub isp: String,
    pub timezone: String,
}

pub const GEO_URL: &str = "http://ip-api.com/json/?fields=status,query,country,countryCode,regionName,city,zip,isp,reverse,timezone";

pub fn parse_geo(v: &Value) -> Result<ExitInfo, String> {
    if v["status"] != "success" {
        return Err("exit IP lookup failed".into());
    }
    let s = |k: &str| v[k].as_str().unwrap_or_default().to_string();
    Ok(ExitInfo {
        ip: s("query"),
        country: s("country"),
        country_code: s("countryCode"),
        region: s("regionName"),
        city: s("city"),
        zip: s("zip"),
        host: s("reverse"),
        isp: s("isp"),
        timezone: s("timezone"),
    })
}

/// Public IP and country as seen from outside, through the running core.
pub async fn exit_info(port: u16) -> Result<ExitInfo, String> {
    let client = reqwest::Client::builder()
        .proxy(reqwest::Proxy::all(format!("http://127.0.0.1:{port}")).map_err(|e| e.to_string())?)
        .timeout(Duration::from_secs(8))
        .build()
        .map_err(|e| e.to_string())?;
    let v: Value = client
        .get(GEO_URL)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;
    parse_geo(&v)
}
