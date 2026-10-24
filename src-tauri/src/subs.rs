//! Named subscriptions: fetching, traffic/expiry info (see usage.rs), and periodic refresh.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};

use crate::{
    nodes::{self, Node},
    state::AppState,
    usage::{self, Usage},
};

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Subscription {
    pub id: String,
    pub name: String,
    pub url: String,
    /// Unix seconds of the last successful refresh.
    pub updated_at: Option<u64>,
    pub upload: Option<u64>,
    pub download: Option<u64>,
    pub total: Option<u64>,
    /// Unix seconds.
    pub expire: Option<u64>,
    /// Remaining bytes, for panels that only publish what is left (no total / used split).
    #[serde(default)]
    pub remaining: Option<u64>,
    /// Largest `remaining` seen since the last renewal; stands in for the total when none is published.
    #[serde(default)]
    pub remaining_peak: Option<u64>,
    /// Where the numbers came from: "header" or "remarks".
    #[serde(default)]
    pub info_source: Option<String>,
}

impl Subscription {
    pub fn new(name: String, url: String) -> Self {
        Self { id: uuid::Uuid::new_v4().to_string(), name, url, ..Self::default() }
    }

    /// Replaces the stored usage with a fresh reading (stale numbers must not outlive a panel that stopped sending them).
    fn apply_usage(&mut self, reading: Option<(Usage, &'static str)>) {
        let previous = self.remaining;
        let (u, source) = match reading {
            Some((u, source)) => (u, Some(source.to_string())),
            None => (Usage::default(), None),
        };
        (self.upload, self.download, self.total, self.expire) = (u.upload, u.download, u.total, u.expire);
        self.remaining = u.remaining;
        self.info_source = source;
        self.remaining_peak = match (u.remaining, previous) {
            (None, _) => None,
            // It went up: the plan was renewed, start a new baseline.
            (Some(now), Some(before)) if now > before => Some(now),
            (Some(now), _) => Some(self.remaining_peak.unwrap_or(0).max(now)),
        };
    }
}

#[derive(Serialize)]
pub struct ImportResult {
    pub added: usize,
    pub errors: Vec<String>,
}

fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

/// Panels pick the response format from the User-Agent: a "sing-box" UA often gets an empty body or a
/// full config we cannot import, while a v2rayN-style UA gets the plain base64 link list. Try them in turn.
const USER_AGENTS: [&str; 3] = ["v2rayN/6.45", "Vanish/0.1", "clash.meta"];

async fn fetch(url: &str, user_agent: &str) -> Result<(String, Option<String>), String> {
    let resp = reqwest::Client::builder()
        .user_agent(user_agent)
        .timeout(Duration::from_secs(20))
        .build()
        .map_err(|e| e.to_string())?
        .get(url)
        .send()
        .await
        .and_then(|r| r.error_for_status())
        .map_err(|e| e.to_string())?;
    let info = ["subscription-userinfo", "x-subscription-userinfo"]
        .iter()
        .find_map(|h| resp.headers().get(*h))
        .and_then(|v| v.to_str().ok())
        .map(str::to_string);
    let body = resp.text().await.map_err(|e| e.to_string())?;
    Ok((body, info))
}

/// Downloads `url` and stores it as subscription `sub_id` (creating nothing: the subscription must exist).
/// Nodes that survive the refresh keep their id and latency so the current selection stays valid.
pub async fn refresh(app: &AppHandle, sub_id: &str) -> Result<ImportResult, String> {
    let state = app.state::<AppState>();
    let mut sub = state.subs.lock().unwrap().iter().find(|s| s.id == sub_id).cloned().ok_or("unknown subscription")?;
    let mut last_err = String::from("empty response from server");
    let mut found = None;
    let mut header = None; // keep the first usage header any attempt returned
    for ua in USER_AGENTS {
        match fetch(&sub.url, ua).await {
            Ok((body, info)) => {
                header = header.or(info);
                let (parsed, errors) = nodes::parse_text(&body);
                if !parsed.is_empty() {
                    found = Some((parsed, errors));
                    break;
                }
                last_err = if body.trim().is_empty() {
                    "empty response from server".into()
                } else {
                    format!("no usable nodes in subscription ({} entries skipped)", errors.len())
                };
            }
            Err(e) => last_err = e,
        }
    }
    let Some((mut parsed, errors)) = found else { return Err(last_err) };
    sub.apply_usage(usage::combine(header.as_deref().map(usage::from_header), usage::from_names(parsed.iter().map(|n| n.name.as_str()))));
    sub.updated_at = Some(now());
    let added = parsed.len();
    {
        let mut nodes = state.nodes.lock().unwrap();
        let old: Vec<Node> = nodes.iter().filter(|n| n.subscription.as_deref() == Some(sub_id)).cloned().collect();
        for n in &mut parsed {
            n.subscription = Some(sub_id.to_string());
            if let Some(o) = old.iter().find(|o| o.server == n.server && o.port == n.port && o.protocol == n.protocol) {
                n.id = o.id.clone();
                n.latency = o.latency;
                n.delay = o.delay;
                n.up_total = o.up_total;
                n.down_total = o.down_total;
            }
        }
        nodes.retain(|n| n.subscription.as_deref() != Some(sub_id));
        nodes.extend(parsed);
    }
    if let Some(s) = state.subs.lock().unwrap().iter_mut().find(|s| s.id == sub_id) {
        *s = sub;
    }
    state.save_nodes()?;
    state.save_subs()?;
    crate::commands::ensure_selection(&state)?;
    let _ = app.emit("nodes-changed", ());
    Ok(ImportResult { added, errors })
}

/// Refreshes subscriptions that are older than `sub_refresh_hours`; checked every 30 minutes.
pub async fn refresh_loop(app: AppHandle) {
    tokio::time::sleep(Duration::from_secs(8)).await;
    loop {
        let (hours, due) = {
            let state = app.state::<AppState>();
            let hours = state.settings.lock().unwrap().sub_refresh_hours;
            let due: Vec<String> = state
                .subs
                .lock()
                .unwrap()
                .iter()
                .filter(|s| now().saturating_sub(s.updated_at.unwrap_or(0)) >= hours as u64 * 3600)
                .map(|s| s.id.clone())
                .collect();
            (hours, due)
        };
        if hours > 0 {
            for id in due {
                if let Err(e) = refresh(&app, &id).await {
                    eprintln!("subscription refresh failed: {e}");
                }
            }
        }
        tokio::time::sleep(Duration::from_secs(1800)).await;
    }
}
