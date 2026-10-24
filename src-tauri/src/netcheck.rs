//! Network tools: speed test (ping / download / upload) and a leak check (IP, DNS, IPv6).
//! Both go through the running tunnel when connected and straight out otherwise.

use std::{
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc,
    },
    time::Duration,
};

use serde::Serialize;
use serde_json::Value;
use tauri::{AppHandle, Emitter, Manager};
use tokio::time::{sleep, Instant};

use crate::{
    engine::{self, ExitInfo},
    state::{AppState, Mode},
};

const DOWN_URL: &str = "https://speed.cloudflare.com/__down?bytes=100000000";
const UP_URL: &str = "https://speed.cloudflare.com/__up";
const PHASE_SECS: f64 = 7.0;
/// Ramp-up (TCP slow start, TLS) is excluded from the reported speed.
const WARMUP_SECS: f64 = 1.0;
static SPEED_RUNNING: AtomicBool = AtomicBool::new(false);

fn client(app: &AppHandle, timeout: Duration) -> Result<(reqwest::Client, bool), String> {
    let state = app.state::<AppState>();
    let via_tunnel = state.active_node.lock().unwrap().is_some();
    let mut b = reqwest::Client::builder().timeout(timeout).no_proxy();
    if via_tunnel {
        let port = state.active_ports.lock().unwrap().0;
        b = b.proxy(reqwest::Proxy::all(format!("http://127.0.0.1:{port}")).map_err(|e| e.to_string())?);
    }
    Ok((b.build().map_err(|e| e.to_string())?, via_tunnel))
}

#[derive(Serialize, Clone)]
struct SpeedEvent {
    phase: &'static str,
    /// ms for "ping", Mbit/s for "download" / "upload".
    value: f64,
}

#[derive(Serialize, Clone, Default)]
pub struct SpeedResult {
    pub via_tunnel: bool,
    pub ping_ms: f64,
    pub jitter_ms: f64,
    pub download_mbps: f64,
    pub upload_mbps: f64,
}

struct Guard;
impl Drop for Guard {
    fn drop(&mut self) {
        SPEED_RUNNING.store(false, Ordering::SeqCst);
    }
}

pub async fn speed_test(app: &AppHandle) -> Result<SpeedResult, String> {
    if SPEED_RUNNING.swap(true, Ordering::SeqCst) {
        return Err("a speed test is already running".into());
    }
    let _guard = Guard;
    let (client, via_tunnel) = client(app, Duration::from_secs(30))?;
    let mut result = SpeedResult { via_tunnel, ..Default::default() };

    // --- latency: reuse one connection, so the numbers are request round-trips, not handshakes
    let ping_url = "https://speed.cloudflare.com/cdn-cgi/trace";
    client.get(ping_url).send().await.map_err(|e| format!("speed server unreachable: {e}"))?;
    let mut samples = vec![];
    for _ in 0..6 {
        let t = Instant::now();
        if client.get(ping_url).send().await.is_ok() {
            samples.push(t.elapsed().as_secs_f64() * 1000.0);
        }
    }
    if !samples.is_empty() {
        result.ping_ms = samples.iter().sum::<f64>() / samples.len() as f64;
        result.jitter_ms = samples.windows(2).map(|w| (w[1] - w[0]).abs()).sum::<f64>() / (samples.len().max(2) - 1) as f64;
    }
    let _ = app.emit("speedtest", SpeedEvent { phase: "ping", value: result.ping_ms });

    // --- download / upload: several parallel streams for PHASE_SECS
    result.download_mbps = transfer(app, &client, "download").await;
    result.upload_mbps = transfer(app, &client, "upload").await;
    let _ = app.emit("speedtest", SpeedEvent { phase: "done", value: 0.0 });
    Ok(result)
}

async fn transfer(app: &AppHandle, client: &reqwest::Client, phase: &'static str) -> f64 {
    let bytes = Arc::new(AtomicU64::new(0));
    let stop = Arc::new(AtomicBool::new(false));
    let streams = if phase == "download" { 4 } else { 6 };
    let tasks: Vec<_> = (0..streams)
        .map(|_| {
            let (client, bytes, stop) = (client.clone(), bytes.clone(), stop.clone());
            tauri::async_runtime::spawn(async move {
                if phase == "download" {
                    while !stop.load(Ordering::Relaxed) {
                        let Ok(mut resp) = client.get(DOWN_URL).send().await else { break };
                        while !stop.load(Ordering::Relaxed) {
                            match resp.chunk().await {
                                Ok(Some(c)) => {
                                    bytes.fetch_add(c.len() as u64, Ordering::Relaxed);
                                }
                                _ => break,
                            }
                        }
                    }
                } else {
                    let body = vec![0u8; 512 * 1024];
                    while !stop.load(Ordering::Relaxed) {
                        if client.post(UP_URL).body(body.clone()).send().await.is_err() {
                            break;
                        }
                        bytes.fetch_add(body.len() as u64, Ordering::Relaxed);
                    }
                }
            })
        })
        .collect();

    let start = Instant::now();
    let (mut last_t, mut last_b) = (start, 0u64);
    let mut at_warmup: Option<(f64, u64)> = None;
    while start.elapsed().as_secs_f64() < PHASE_SECS {
        sleep(Duration::from_millis(250)).await;
        let (now, b) = (Instant::now(), bytes.load(Ordering::Relaxed));
        let dt = now.duration_since(last_t).as_secs_f64().max(0.001);
        let _ = app.emit("speedtest", SpeedEvent { phase, value: (b - last_b) as f64 * 8.0 / dt / 1e6 });
        (last_t, last_b) = (now, b);
        if at_warmup.is_none() && start.elapsed().as_secs_f64() >= WARMUP_SECS {
            at_warmup = Some((start.elapsed().as_secs_f64(), b));
        }
    }
    stop.store(true, Ordering::Relaxed);
    let (elapsed, total) = (start.elapsed().as_secs_f64(), bytes.load(Ordering::Relaxed));
    for t in tasks {
        t.abort();
    }
    let (t0, b0) = at_warmup.unwrap_or((0.0, 0));
    let mbps = if elapsed > t0 { (total - b0) as f64 * 8.0 / (elapsed - t0) / 1e6 } else { 0.0 };
    let _ = app.emit("speedtest", SpeedEvent { phase, value: mbps });
    mbps
}

// ---------------------------------------------------------------- leak check

#[derive(Serialize, Clone)]
pub struct Check {
    pub label: &'static str,
    /// "ok" | "warn" | "fail" | "info"
    pub status: &'static str,
    pub detail: String,
}

#[derive(Serialize, Clone)]
pub struct Diagnostics {
    pub connected: bool,
    pub checks: Vec<Check>,
}

async fn geo(client: &reqwest::Client) -> Option<ExitInfo> {
    let v: Value = client.get(engine::GEO_URL).send().await.ok()?.json().await.ok()?;
    engine::parse_geo(&v).ok()
}

/// Name of the DNS server that resolved a unique hostname for us (ip-api's EDNS probe).
async fn resolver(client: &reqwest::Client) -> Option<String> {
    let v: Value = client.get("http://edns.ip-api.com/json").send().await.ok()?.json().await.ok()?;
    v["dns"]["geo"].as_str().map(str::to_string)
}

fn check(label: &'static str, status: &'static str, detail: impl Into<String>) -> Check {
    Check { label, status, detail: detail.into() }
}

pub async fn diagnostics(app: &AppHandle) -> Result<Diagnostics, String> {
    let state = app.state::<AppState>();
    let mode = state.settings.lock().unwrap().mode;
    let (tunnel_client, connected) = client(app, Duration::from_secs(8))?;
    let direct_client = reqwest::Client::builder().no_proxy().timeout(Duration::from_secs(8)).build().map_err(|e| e.to_string())?;
    // IPv6 only answers if the machine has a working v6 route.
    let v6_client = reqwest::Client::builder().no_proxy().timeout(Duration::from_secs(4)).build().map_err(|e| e.to_string())?;

    let (tunnel, direct, dns_tunnel, dns_direct, v6) = tokio::join!(
        async { if connected { geo(&tunnel_client).await } else { None } },
        geo(&direct_client),
        async { if connected { resolver(&tunnel_client).await } else { None } },
        resolver(&direct_client),
        async { v6_client.get("https://api6.ipify.org").send().await.ok()?.text().await.ok() },
    );

    let mut checks = vec![];
    let fmt = |g: &ExitInfo| format!("{} ({})", g.ip, g.country_code);

    if !connected {
        checks.push(check("Tunnel", "info", "Not connected. Showing your real network below."));
        checks.push(match &direct {
            Some(g) => check("Your IP", "info", format!("{} · {}, {}", fmt(g), g.city, g.country)),
            None => check("Your IP", "fail", "Could not reach the lookup service."),
        });
        if let Some(d) = &dns_direct {
            checks.push(check("DNS resolver", "info", d.clone()));
        }
        return Ok(Diagnostics { connected, checks });
    }

    match &tunnel {
        Some(g) => checks.push(check("Tunnel", "ok", format!("Exit {} · {}, {}", fmt(g), g.city, g.country))),
        None => checks.push(check("Tunnel", "fail", "No answer through the proxy. The node may be down.")),
    }

    // IP: with TUN every connection should exit via the tunnel; with a system proxy only proxy-aware apps do.
    if let (Some(t), Some(d)) = (&tunnel, &direct) {
        let same = t.ip == d.ip;
        checks.push(match mode {
            Mode::Tun if same => check("IP leak", "ok", "All traffic leaves through the tunnel."),
            Mode::Tun => check("IP leak", "fail", format!("Direct connections still exit from {}.", fmt(d))),
            _ if same => check("IP leak", "info", "Direct and tunnel IP are identical (the node may be on this network)."),
            _ => check("IP leak", "info", format!("Apps that ignore the proxy settings connect from {}. Use TUN to cover them.", fmt(d))),
        });
    }

    // DNS: the resolver the system uses, compared with the resolver behind the tunnel.
    match (&dns_direct, &dns_tunnel) {
        (Some(sys), Some(tun)) => {
            let same = sys == tun;
            checks.push(match mode {
                Mode::Tun if same => check("DNS leak", "ok", format!("Queries are answered by {tun}.")),
                Mode::Tun => check("DNS leak", "fail", format!("System DNS goes to {sys}, not the tunnel's {tun}.")),
                _ if same => check("DNS leak", "ok", format!("Queries are answered by {tun}.")),
                _ => check("DNS leak", "info", format!("Proxy-aware apps resolve via {tun}; apps outside the proxy use {sys}.")),
            });
        }
        _ => checks.push(check("DNS leak", "warn", "Could not determine the DNS resolver.")),
    }

    checks.push(match v6 {
        Some(ip) if mode == Mode::Tun => check("IPv6", "info", format!("IPv6 address {ip} is routed through the TUN interface.")),
        Some(ip) => check("IPv6", "warn", format!("IPv6 is reachable outside the tunnel ({ip}). Sites may see it instead of your exit IP.")),
        None => check("IPv6", "ok", "No IPv6 connectivity, nothing to leak."),
    });

    Ok(Diagnostics { connected, checks })
}
