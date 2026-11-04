//! "Real delay" test (like URL test in Nekoray): a throw-away headless sing-box holds every node as an
//! outbound and the Clash API's delay endpoint makes a real HTTP(S) request through each one.
//! Nodes that sing-box cannot dial (XHTTP, mKCP, ...) sit behind a short-lived Xray process, see `crate::xray`.

use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tokio::{sync::Semaphore, time::sleep};

use crate::{config, nodes::Node, state::AppState};

static RUNNING: AtomicBool = AtomicBool::new(false);

#[derive(Serialize, Clone)]
struct DelayEvent {
    id: String,
    /// ms, or -1 when the node did not answer.
    delay: i32,
}

struct Guard;
impl Drop for Guard {
    fn drop(&mut self) {
        RUNNING.store(false, Ordering::SeqCst);
    }
}

pub async fn real_delay(app: &AppHandle, ids: Vec<String>) -> Result<(), String> {
    if RUNNING.swap(true, Ordering::SeqCst) {
        return Err("a delay test is already running".into());
    }
    let _guard = Guard;
    // The probe starts its own sing-box process, which Android does not allow; libbox is a single in-process core.
    #[cfg(target_os = "android")]
    return real_delay_android(app, ids).await;
    #[cfg(not(target_os = "android"))]
    real_delay_desktop(app, ids).await
}

fn record(app: &AppHandle, id: String, delay: i32) {
    let state = app.state::<AppState>();
    if let Some(n) = state.nodes.lock().unwrap().iter_mut().find(|n| n.id == id) {
        n.delay = Some(delay);
    }
    let _ = app.emit("node-delay", DelayEvent { id, delay });
}

/// One delay request through the Clash API; -1 on timeout or error.
async fn delay_once(client: &reqwest::Client, api_port: u16, tag: &str, url: &str) -> Result<i32, String> {
    let resp = client
        .get(format!("http://127.0.0.1:{api_port}/proxies/{tag}/delay"))
        .query(&[("url", url), ("timeout", "6000")])
        .timeout(Duration::from_secs(9))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let ok = resp.status().is_success();
    let body = resp.json::<serde_json::Value>().await.map_err(|e| e.to_string())?;
    if ok {
        body["delay"].as_i64().map(|d| d as i32).ok_or_else(|| "no delay in answer".to_string())
    } else {
        Err(body["message"].as_str().unwrap_or("failed").to_string())
    }
}

/// The Clash API opens a brand-new connection for every request, so one measurement carries everything that
/// happens once per connection: resolving the server, TCP slow start, the transport handshake and the first
/// TLS flight. A single cold sample is also at the mercy of one lost packet, which is how a healthy server ends
/// up as "timeout" here and 150 ms in v2rayNG. Two attempts, the better one reported, give the steady figure
/// those clients show (and a failed first try gets its second chance).
async fn measure(client: &reqwest::Client, api_port: u16, tag: &str, url: &str) -> (i32, Option<String>) {
    let first = delay_once(client, api_port, tag, url).await;
    let second = delay_once(client, api_port, tag, url).await;
    match (first, second) {
        (Ok(a), Ok(b)) => (a.min(b), None),
        (Ok(a), Err(_)) | (Err(_), Ok(a)) => (a, None),
        (Err(_), Err(e)) => (-1, Some(e)),
    }
}

/// Times every `(node id, outbound tag)` pair with bounded concurrency (hundreds of simultaneous handshakes
/// would skew the numbers) and publishes each result as it lands.
async fn run_delays(app: &AppHandle, client: &reqwest::Client, api_port: u16, tags: Vec<(String, String)>, url: &str, concurrency: usize) {
    let gate = Arc::new(Semaphore::new(concurrency));
    let handles: Vec<_> = tags
        .into_iter()
        .map(|(id, tag)| {
            let (client, gate, app, url) = (client.clone(), gate.clone(), app.clone(), url.to_string());
            tauri::async_runtime::spawn(async move {
                let _permit = gate.acquire().await;
                let (delay, err) = measure(&client, api_port, &tag, &url).await;
                if let Some(e) = err {
                    // The reason a node timed out (dial error, handshake, ...) is what tells a bad node from a bad client.
                    let name = app.state::<AppState>().nodes.lock().unwrap().iter().find(|n| n.id == id).map(|n| n.name.clone()).unwrap_or_default();
                    let _ = app.emit("core-log", format!("delay {name}: {e}"));
                }
                record(&app, id, delay);
            })
        })
        .collect();
    for h in handles {
        let _ = h.await;
    }
}

fn log_xray(app: &AppHandle, x: &crate::xray::Running) {
    for line in x.log_tail(12) {
        let _ = app.emit("core-log", line);
    }
}

/// Starts Xray for the Xray-only targets. When it cannot start those nodes are reported as failed and dropped.
async fn xray_for(app: &AppHandle, targets: &mut Vec<Node>) -> Option<crate::xray::Running> {
    match crate::xray::start(app, targets).await {
        Ok(running) => running,
        Err(e) => {
            let _ = app.emit("core-log", format!("xray: {e}"));
            let (broken, ok): (Vec<Node>, Vec<Node>) = std::mem::take(targets).into_iter().partition(|n| n.xray.is_some());
            for n in broken {
                record(app, n.id, -1);
            }
            *targets = ok;
            None
        }
    }
}

/// Android: times every node through the Clash API of an in-process core.
/// Connected: the running core carries every node as `p-<id>` (see `config::android_extras`).
/// Not connected: a short-lived headless core is started just for the test (no VPN, no permission prompt).
#[cfg(target_os = "android")]
async fn real_delay_android(app: &AppHandle, ids: Vec<String>) -> Result<(), String> {
    use crate::engine;
    let state = app.state::<AppState>();
    let test_url = state.settings.lock().unwrap().ping_url.clone();
    let client = reqwest::Client::builder().no_proxy().build().map_err(|e| e.to_string())?;

    // (api port, tag per node id)
    let connected = state.active_node.lock().unwrap().is_some();
    let (api_port, tags, probe_xray): (u16, Vec<(String, String)>, Option<crate::xray::Running>) = if connected {
        (state.active_ports.lock().unwrap().1, ids.iter().map(|id| (id.clone(), format!("p-{id}"))).collect(), None)
    } else {
        let mut targets: Vec<Node> = state.nodes.lock().unwrap().iter().filter(|n| ids.contains(&n.id)).cloned().collect();
        if targets.is_empty() {
            return Ok(());
        }
        let xray = xray_for(app, &mut targets).await;
        if targets.is_empty() {
            return state.save_nodes();
        }
        let ports = xray.as_ref().map(|x| x.ports.clone()).unwrap_or_default();
        let api_port = engine::free_port(19190, None);
        let refs: Vec<&Node> = targets.iter().collect();
        let path = state.data_dir.join("probe.json");
        std::fs::write(&path, serde_json::to_vec(&config::probe(&refs, api_port, &ports)).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
        if let Err(e) = crate::vpn::start(app, &path, true).await {
            if let Some(x) = xray {
                x.kill().await;
            }
            return Err(e);
        }

        let mut ready = false;
        for _ in 0..80 {
            if client.get(format!("http://127.0.0.1:{api_port}/version")).timeout(Duration::from_millis(300)).send().await.is_ok() {
                ready = true;
                break;
            }
            if let Some(err) = crate::vpn::status(app).await?.error {
                let _ = crate::vpn::stop(app).await;
                if let Some(x) = xray {
                    x.kill().await;
                }
                return Err(format!("core exited:\n{err}"));
            }
            sleep(Duration::from_millis(250)).await;
        }
        if !ready {
            let _ = crate::vpn::stop(app).await;
            if let Some(x) = xray {
                x.kill().await;
            }
            return Err("probe core did not become ready".into());
        }
        (api_port, targets.iter().enumerate().map(|(i, n)| (n.id.clone(), format!("n{i}"))).collect(), xray)
    };

    run_delays(app, &client, api_port, tags, &test_url, 6).await;
    if !connected {
        let _ = crate::vpn::stop(app).await;
    }
    if let Some(x) = probe_xray {
        log_xray(app, &x);
        x.kill().await;
    }
    state.save_nodes()
}

#[cfg(not(target_os = "android"))]
async fn real_delay_desktop(app: &AppHandle, ids: Vec<String>) -> Result<(), String> {
    use std::process::Stdio;
    use tokio::process::Command;

    use crate::engine;

    let state = app.state::<AppState>();

    let mut targets: Vec<Node> = state.nodes.lock().unwrap().iter().filter(|n| ids.contains(&n.id)).cloned().collect();
    if targets.is_empty() {
        return Ok(());
    }
    let (test_url, bin) = {
        let url = state.settings.lock().unwrap().ping_url.clone();
        (url, engine::resolve_binary(app, &state))
    };
    let xray = xray_for(app, &mut targets).await;
    if targets.is_empty() {
        return state.save_nodes();
    }
    let ports: HashMap<String, u16> = xray.as_ref().map(|x| x.ports.clone()).unwrap_or_default();
    let kill_xray = |x: Option<crate::xray::Running>| async move {
        if let Some(x) = x {
            x.kill().await;
        }
    };

    let dir = state.data_dir.join("probe");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let api_port = engine::free_port(19190, None);
    let refs: Vec<&Node> = targets.iter().collect();
    std::fs::write(dir.join("config.json"), serde_json::to_vec(&config::probe(&refs, api_port, &ports)).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    let log_path = dir.join("core.log");
    let log = std::fs::File::create(&log_path).map_err(|e| e.to_string())?;

    let mut cmd = Command::new(&bin);
    cmd.args(["run", "-c"])
        .arg(dir.join("config.json"))
        .arg("-D")
        .arg(&dir)
        .stdout(Stdio::null())
        .stderr(Stdio::from(log))
        .kill_on_drop(true);
    #[cfg(windows)]
    cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            kill_xray(xray).await;
            return Err(format!("cannot start {}: {e}", bin.display()));
        }
    };

    let client = reqwest::Client::builder().no_proxy().build().map_err(|e| e.to_string())?;
    let base = format!("http://127.0.0.1:{api_port}");
    let mut ready = false;
    for _ in 0..40 {
        if let Ok(Some(_)) = child.try_wait() {
            let tail = std::fs::read_to_string(&log_path).unwrap_or_default();
            let tail: Vec<&str> = tail.lines().rev().take(3).collect();
            kill_xray(xray).await;
            return Err(format!("probe core exited: {}", tail.into_iter().rev().collect::<Vec<_>>().join(" | ")));
        }
        if client.get(format!("{base}/version")).timeout(Duration::from_millis(300)).send().await.is_ok() {
            ready = true;
            break;
        }
        sleep(Duration::from_millis(150)).await;
    }
    if !ready {
        let _ = child.kill().await;
        kill_xray(xray).await;
        return Err("probe core did not become ready".into());
    }

    let tags: Vec<(String, String)> = targets.iter().enumerate().map(|(i, n)| (n.id.clone(), format!("n{i}"))).collect();
    run_delays(app, &client, api_port, tags, &test_url, 8).await;
    let _ = child.kill().await;
    if let Some(x) = &xray {
        log_xray(app, x);
    }
    kill_xray(xray).await;
    state.save_nodes()
}
