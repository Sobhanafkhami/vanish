//! Named subscriptions: fetching, usage info from `subscription-userinfo`, and periodic refresh.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};

use crate::{
    nodes::{self, Node},
    state::AppState,
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
}

impl Subscription {
    pub fn new(name: String, url: String) -> Self {
        Self { id: uuid::Uuid::new_v4().to_string(), name, url, ..Self::default() }
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

/// `upload=1; download=2; total=3; expire=4`
fn parse_userinfo(h: &str, sub: &mut Subscription) {
    for part in h.split(';') {
        let Some((k, v)) = part.split_once('=') else { continue };
        let Ok(v) = v.trim().parse::<u64>() else { continue };
        match k.trim() {
            "upload" => sub.upload = Some(v),
            "download" => sub.download = Some(v),
            "total" => sub.total = Some(v),
            "expire" => sub.expire = Some(v).filter(|e| *e > 0),
            _ => {}
        }
    }
}

async fn fetch(url: &str) -> Result<(String, Option<String>), String> {
    let resp = reqwest::Client::builder()
        .user_agent("Vanish/0.1 sing-box")
        .timeout(Duration::from_secs(20))
        .build()
        .map_err(|e| e.to_string())?
        .get(url)
        .send()
        .await
        .and_then(|r| r.error_for_status())
        .map_err(|e| e.to_string())?;
    let info = resp.headers().get("subscription-userinfo").and_then(|v| v.to_str().ok()).map(str::to_string);
    let body = resp.text().await.map_err(|e| e.to_string())?;
    Ok((body, info))
}

/// Downloads `url` and stores it as subscription `sub_id` (creating nothing: the subscription must exist).
/// Nodes that survive the refresh keep their id and latency so the current selection stays valid.
pub async fn refresh(app: &AppHandle, sub_id: &str) -> Result<ImportResult, String> {
    let state = app.state::<AppState>();
    let mut sub = state.subs.lock().unwrap().iter().find(|s| s.id == sub_id).cloned().ok_or("unknown subscription")?;
    let (body, info) = fetch(&sub.url).await?;
    let (mut parsed, errors) = nodes::parse_text(&body);
    if parsed.is_empty() {
        return Err(format!("no usable nodes in subscription ({} skipped)", errors.len()));
    }
    if let Some(info) = info {
        parse_userinfo(&info, &mut sub);
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_userinfo_header() {
        let mut s = Subscription::new("t".into(), "https://x".into());
        parse_userinfo("upload=10; download=20; total=100; expire=1800000000", &mut s);
        assert_eq!((s.upload, s.download, s.total, s.expire), (Some(10), Some(20), Some(100), Some(1_800_000_000)));
        parse_userinfo("expire=0", &mut s);
        assert_eq!(s.expire, None); // 0 means "never expires"
    }
}
