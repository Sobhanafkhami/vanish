//! Network tools: speed test (ping / download / upload) and a leak check (IP, DNS, IPv6).
//! Both go through the running tunnel when connected and straight out otherwise.

use std::{
    collections::VecDeque,
    sync::{
        atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
        Arc, Mutex,
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

/// Cloudflare's speed endpoints answer HTTP 403 above this size (a 100 MB request, which an earlier
/// version used, is refused: that was the "0 Mbit/s" bug).
const DOWN_BYTES: u64 = 25_000_000;
const SERVER: &str = "https://speed.cloudflare.com";
const PHASE_SECS: f64 = 8.0;
/// Ramp-up (TCP slow start, TLS) is excluded from the reported speed.
const WARMUP_SECS: f64 = 1.5;
/// The live reading is the average of the last second, which is steady yet still responsive.
const WINDOW_SECS: f64 = 1.0;
const DOWN_STREAMS: usize = 8;
const UP_STREAMS: usize = 6;
const UP_REQUEST_BYTES: usize = 4 * 1024 * 1024;
const UP_CHUNK: usize = 32 * 1024;
static ZEROS: [u8; UP_CHUNK] = [0; UP_CHUNK];
static SPEED_RUNNING: AtomicBool = AtomicBool::new(false);

fn client(app: &AppHandle, timeout: Duration) -> Result<(reqwest::Client, bool), String> {
    let state = app.state::<AppState>();
    let via_tunnel = state.active_node.lock().unwrap().is_some();
    // One TCP connection per stream, like a browser speed test.
    let mut b = reqwest::Client::builder().timeout(timeout).no_proxy().http1_only();
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
    /// The phase finished; `value` is its final result.
    done: bool,
}

#[derive(Serialize, Clone, Default)]
pub struct SpeedResult {
    pub via_tunnel: bool,
    pub ping_ms: f64,
    pub jitter_ms: f64,
    pub download_mbps: f64,
    pub upload_mbps: f64,
    pub server: String,
    pub ip: String,
    pub isp: String,
    pub location: String,
    /// Why a phase has no result (e.g. blocked server), so a missing number is never silent.
    pub notes: Vec<String>,
}

struct Guard;
impl Drop for Guard {
    fn drop(&mut self) {
        SPEED_RUNNING.store(false, Ordering::SeqCst);
    }
}

fn trace_field(trace: &str, key: &str) -> Option<String> {
    trace.lines().find_map(|l| l.strip_prefix(key)?.strip_prefix('=')).map(str::to_string)
}

pub async fn speed_test(app: &AppHandle) -> Result<SpeedResult, String> {
    if SPEED_RUNNING.swap(true, Ordering::SeqCst) {
        return Err("a speed test is already running".into());
    }
    let _guard = Guard;
    let (client, via_tunnel) = client(app, Duration::from_secs(60))?;
    let mut result = SpeedResult { via_tunnel, ..Default::default() };
    let emit = |phase: &'static str, value: f64, done: bool| {
        let _ = app.emit("speedtest", SpeedEvent { phase, value, done });
    };

    // --- server + who we are (shown like Speedtest.net's server / ISP line)
    let trace_url = format!("{SERVER}/cdn-cgi/trace");
    let trace = client
        .get(&trace_url)
        .send()
        .await
        .and_then(|r| r.error_for_status())
        .map_err(|e| format!("speed server unreachable: {e}"))?
        .text()
        .await
        .unwrap_or_default();
    result.server = match trace_field(&trace, "colo") {
        Some(colo) => format!("Cloudflare · {colo}"),
        None => "Cloudflare".into(),
    };
    if let Some(g) = geo(&client).await {
        result.location = [g.city.as_str(), g.country.as_str()].iter().filter(|s| !s.is_empty()).copied().collect::<Vec<_>>().join(", ");
        (result.ip, result.isp) = (g.ip, g.isp);
    }

    // --- latency: reuse one connection, so the numbers are request round-trips, not handshakes
    let mut samples = vec![];
    for _ in 0..9 {
        let t = Instant::now();
        if client.get(&trace_url).send().await.is_ok_and(|r| r.status().is_success()) {
            samples.push(t.elapsed().as_secs_f64() * 1000.0);
        }
        emit("ping", samples.last().copied().unwrap_or(0.0), false);
    }
    if samples.len() > 1 {
        samples.remove(0); // the first request may still be opening the connection
        result.jitter_ms = samples.windows(2).map(|w| (w[1] - w[0]).abs()).sum::<f64>() / (samples.len().max(2) - 1) as f64;
        let mut sorted = samples.clone();
        sorted.sort_by(|a, b| a.total_cmp(b));
        result.ping_ms = sorted[sorted.len() / 2];
    }
    emit("ping", result.ping_ms, true);

    // --- download / upload: parallel streams for PHASE_SECS
    for phase in ["download", "upload"] {
        let on_sample = |value: f64, done: bool| {
            let _ = app.emit("speedtest", SpeedEvent { phase, value, done });
        };
        match transfer(&client, phase, on_sample).await {
            Ok(mbps) if phase == "download" => result.download_mbps = mbps,
            Ok(mbps) => result.upload_mbps = mbps,
            Err(e) => result.notes.push(format!("{phase} failed: {e}")),
        }
    }
    if result.ping_ms == 0.0 && result.download_mbps == 0.0 && result.upload_mbps == 0.0 {
        return Err(result.notes.join("; ").if_empty("speed test failed"));
    }
    emit("done", 0.0, true);
    Ok(result)
}

trait IfEmpty {
    fn if_empty(self, fallback: &str) -> String;
}
impl IfEmpty for String {
    fn if_empty(self, fallback: &str) -> String {
        if self.is_empty() { fallback.into() } else { self }
    }
}

struct Shared {
    bytes: AtomicU64,
    /// Streams that are still running.
    alive: AtomicUsize,
    stop: AtomicBool,
    error: Mutex<Option<String>>,
}

impl Shared {
    fn fail(&self, e: impl ToString) {
        self.error.lock().unwrap().get_or_insert_with(|| e.to_string());
    }
}

pub(crate) async fn transfer(client: &reqwest::Client, phase: &'static str, emit: impl Fn(f64, bool)) -> Result<f64, String> {
    let streams = if phase == "download" { DOWN_STREAMS } else { UP_STREAMS };
    let shared = Arc::new(Shared { bytes: AtomicU64::new(0), alive: AtomicUsize::new(streams), stop: AtomicBool::new(false), error: Mutex::new(None) });
    let tasks: Vec<_> = (0..streams)
        .map(|_| {
            let (client, sh) = (client.clone(), shared.clone());
            tauri::async_runtime::spawn(async move {
                if phase == "download" {
                    download_stream(&client, &sh).await
                } else {
                    upload_stream(&client, &sh).await
                }
                sh.alive.fetch_sub(1, Ordering::Relaxed);
            })
        })
        .collect();

    let start = Instant::now();
    // (seconds since start, bytes) points for the sliding window.
    let mut points: VecDeque<(f64, u64)> = VecDeque::from([(0.0, 0)]);
    let mut at_warmup: Option<(f64, u64)> = None;
    loop {
        sleep(Duration::from_millis(200)).await;
        let (t, b) = (start.elapsed().as_secs_f64(), shared.bytes.load(Ordering::Relaxed));
        points.push_back((t, b));
        while points.len() > 2 && t - points[0].0 > WINDOW_SECS {
            points.pop_front();
        }
        let (t0, b0) = points[0];
        emit((b - b0) as f64 * 8.0 / (t - t0).max(0.001) / 1e6, false);
        if at_warmup.is_none() && t >= WARMUP_SECS {
            at_warmup = Some((t, b));
        }
        let all_dead = shared.alive.load(Ordering::Relaxed) == 0;
        if t >= PHASE_SECS || all_dead || (t >= 3.0 && b == 0 && shared.error.lock().unwrap().is_some()) {
            break;
        }
    }
    shared.stop.store(true, Ordering::Relaxed);
    let (elapsed, total) = (start.elapsed().as_secs_f64(), shared.bytes.load(Ordering::Relaxed));
    for t in tasks {
        t.abort();
    }
    if total == 0 {
        let why = shared.error.lock().unwrap().clone();
        return Err(why.unwrap_or_else(|| "no data received".into()));
    }
    // Steady-state average once the connections have ramped up; a very short run falls back to the whole run.
    let mbps = match at_warmup {
        Some((tw, bw)) if elapsed - tw >= 1.0 => (total - bw) as f64 * 8.0 / (elapsed - tw) / 1e6,
        _ => total as f64 * 8.0 / elapsed.max(0.001) / 1e6,
    };
    emit(mbps, true);
    Ok(mbps)
}

async fn download_stream(client: &reqwest::Client, sh: &Shared) {
    let url = format!("{SERVER}/__down?bytes={DOWN_BYTES}");
    while !sh.stop.load(Ordering::Relaxed) {
        let mut resp = match client.get(&url).send().await {
            Ok(r) if r.status().is_success() => r,
            Ok(r) => return sh.fail(format!("server answered HTTP {}", r.status().as_u16())),
            Err(e) => return sh.fail(e),
        };
        loop {
            match resp.chunk().await {
                Ok(Some(c)) => {
                    sh.bytes.fetch_add(c.len() as u64, Ordering::Relaxed);
                    if sh.stop.load(Ordering::Relaxed) {
                        return;
                    }
                }
                Ok(None) => break,
                Err(e) => return sh.fail(e),
            }
        }
    }
}

async fn upload_stream(client: &reqwest::Client, sh: &Arc<Shared>) {
    let url = format!("{SERVER}/__up");
    while !sh.stop.load(Ordering::Relaxed) {
        // Bytes are counted as the body is handed to the socket, so the reading moves smoothly during a request.
        let counter = sh.clone();
        let chunks = futures_util::stream::iter((0..UP_REQUEST_BYTES / UP_CHUNK).map(move |_| {
            counter.bytes.fetch_add(UP_CHUNK as u64, Ordering::Relaxed);
            Ok::<&'static [u8], std::io::Error>(&ZEROS[..])
        }));
        let sent = client
            .post(&url)
            .header(reqwest::header::CONTENT_LENGTH, UP_REQUEST_BYTES)
            .body(reqwest::Body::wrap_stream(chunks))
            .send()
            .await;
        match sent {
            Ok(r) if r.status().is_success() => {}
            Ok(r) => return sh.fail(format!("server answered HTTP {}", r.status().as_u16())),
            Err(e) => return sh.fail(e),
        }
    }
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
