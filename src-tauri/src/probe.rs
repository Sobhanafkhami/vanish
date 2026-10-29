//! "Real delay" test (like URL test in Nekoray): a throw-away headless sing-box holds every node as an
//! outbound and the Clash API's delay endpoint makes a real HTTP(S) request through each one.

use std::{
    process::Stdio,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tokio::{process::Command, sync::Semaphore, time::sleep};

use crate::{config, engine, nodes::Node, state::AppState};

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
    // The probe starts its own sing-box process, which Android does not allow; libbox is a single in-process core.
    #[cfg(target_os = "android")]
    return real_delay_android(app, ids).await;
    #[cfg(not(target_os = "android"))]
    real_delay_desktop(app, ids).await
}

/// Android: asks the running core (which carries every node as `p-<id>`, see `config::android_extras`) to time each one.
#[cfg(target_os = "android")]
async fn real_delay_android(app: &AppHandle, ids: Vec<String>) -> Result<(), String> {
    if RUNNING.swap(true, Ordering::SeqCst) {
        return Err("a delay test is already running".into());
    }
    let _guard = Guard;
    let state = app.state::<AppState>();
    if state.active_node.lock().unwrap().is_none() {
        return Err("connect first to run the real delay test".into());
    }
    let (api_port, test_url) = (state.active_ports.lock().unwrap().1, state.settings.lock().unwrap().ping_url.clone());
    let client = reqwest::Client::builder().no_proxy().build().map_err(|e| e.to_string())?;
    let gate = Arc::new(Semaphore::new(6));
    let handles: Vec<_> = ids
        .into_iter()
        .map(|id| {
            let (client, gate, app, url) = (client.clone(), gate.clone(), app.clone(), test_url.clone());
            tauri::async_runtime::spawn(async move {
                let _permit = gate.acquire().await;
                let delay = match client
                    .get(format!("http://127.0.0.1:{api_port}/proxies/p-{id}/delay"))
                    .query(&[("url", url.as_str()), ("timeout", "6000")])
                    .timeout(Duration::from_secs(9))
                    .send()
                    .await
                {
                    Ok(r) if r.status().is_success() => {
                        r.json::<serde_json::Value>().await.ok().and_then(|v| v["delay"].as_i64()).map_or(-1, |d| d as i32)
                    }
                    _ => -1,
                };
                let state = app.state::<AppState>();
                if let Some(n) = state.nodes.lock().unwrap().iter_mut().find(|n| n.id == id) {
                    n.delay = Some(delay);
                }
                let _ = app.emit("node-delay", DelayEvent { id, delay });
            })
        })
        .collect();
    for h in handles {
        let _ = h.await;
    }
    state.save_nodes()
}

#[cfg(not(target_os = "android"))]
async fn real_delay_desktop(app: &AppHandle, ids: Vec<String>) -> Result<(), String> {
    if RUNNING.swap(true, Ordering::SeqCst) {
        return Err("a delay test is already running".into());
    }
    let _guard = Guard;
    let state = app.state::<AppState>();

    let targets: Vec<Node> = state.nodes.lock().unwrap().iter().filter(|n| ids.contains(&n.id)).cloned().collect();
    if targets.is_empty() {
        return Ok(());
    }
    let (test_url, bin) = {
        let url = state.settings.lock().unwrap().ping_url.clone();
        (url, engine::resolve_binary(app, &state))
    };

    let dir = state.data_dir.join("probe");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let api_port = engine::free_port(19190, None);
    let refs: Vec<&Node> = targets.iter().collect();
    std::fs::write(dir.join("config.json"), serde_json::to_vec(&config::probe(&refs, api_port)).map_err(|e| e.to_string())?)
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
    let mut child = cmd.spawn().map_err(|e| format!("cannot start {}: {e}", bin.display()))?;

    let client = reqwest::Client::builder().no_proxy().build().map_err(|e| e.to_string())?;
    let base = format!("http://127.0.0.1:{api_port}");
    let mut ready = false;
    for _ in 0..40 {
        if let Ok(Some(_)) = child.try_wait() {
            let tail = std::fs::read_to_string(&log_path).unwrap_or_default();
            let tail: Vec<&str> = tail.lines().rev().take(3).collect();
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
        return Err("probe core did not become ready".into());
    }

    // Bounded concurrency: hundreds of simultaneous TLS handshakes would skew the numbers.
    let gate = Arc::new(Semaphore::new(8));
    let handles: Vec<_> = targets
        .iter()
        .enumerate()
        .map(|(i, n)| {
            let (client, base, url, gate, id) = (client.clone(), base.clone(), test_url.clone(), gate.clone(), n.id.clone());
            let app = app.clone();
            tauri::async_runtime::spawn(async move {
                let _permit = gate.acquire().await;
                let delay = match client
                    .get(format!("{base}/proxies/n{i}/delay"))
                    .query(&[("url", url.as_str()), ("timeout", "6000")])
                    .timeout(Duration::from_secs(9))
                    .send()
                    .await
                {
                    Ok(r) if r.status().is_success() => r
                        .json::<serde_json::Value>()
                        .await
                        .ok()
                        .and_then(|v| v["delay"].as_i64())
                        .map(|d| d as i32)
                        .unwrap_or(-1),
                    _ => -1,
                };
                let state = app.state::<AppState>();
                if let Some(n) = state.nodes.lock().unwrap().iter_mut().find(|n| n.id == id) {
                    n.delay = Some(delay);
                }
                let _ = app.emit("node-delay", DelayEvent { id, delay });
            })
        })
        .collect();
    for h in handles {
        let _ = h.await;
    }
    let _ = child.kill().await;
    state.save_nodes()
}
