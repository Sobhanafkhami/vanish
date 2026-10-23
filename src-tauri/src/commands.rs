//! `#[tauri::command]` surface exposed to the frontend.

use tauri::{AppHandle, State};

use crate::{
    engine::{self, CoreStatus},
    nodes::{self, Node},
    state::{AppState, Settings},
};

#[derive(serde::Serialize)]
pub struct ImportResult {
    pub added: usize,
    pub errors: Vec<String>,
}

/// Selects the first node when nothing is selected yet, so a fresh import is immediately connectable.
fn ensure_selection(state: &AppState) -> Result<(), String> {
    let first = state.nodes.lock().unwrap().first().map(|n| n.id.clone());
    let changed = {
        let mut s = state.settings.lock().unwrap();
        let valid = s.selected_node.as_ref().is_some_and(|id| state.nodes.lock().unwrap().iter().any(|n| &n.id == id));
        if !valid && first.is_some() {
            s.selected_node = first;
            true
        } else {
            false
        }
    };
    if changed { state.save_settings() } else { Ok(()) }
}

#[tauri::command]
pub fn list_nodes(state: State<'_, AppState>) -> Vec<Node> {
    state.nodes.lock().unwrap().clone()
}

#[tauri::command]
pub fn import_text(text: String, state: State<'_, AppState>) -> Result<ImportResult, String> {
    let (parsed, errors) = nodes::parse_text(&text);
    let added = parsed.len();
    state.nodes.lock().unwrap().extend(parsed);
    state.save_nodes()?;
    ensure_selection(&state)?;
    Ok(ImportResult { added, errors })
}

#[tauri::command]
pub async fn import_subscription(url: String, state: State<'_, AppState>) -> Result<ImportResult, String> {
    let body = reqwest::Client::builder()
        .user_agent("Vanish/0.1 sing-box")
        .timeout(std::time::Duration::from_secs(20))
        .build()
        .map_err(|e| e.to_string())?
        .get(&url)
        .send()
        .await
        .and_then(|r| r.error_for_status())
        .map_err(|e| e.to_string())?
        .text()
        .await
        .map_err(|e| e.to_string())?;

    let (mut parsed, errors) = nodes::parse_text(&body);
    for n in &mut parsed {
        n.subscription = Some(url.clone());
    }
    let added = parsed.len();
    {
        // Re-importing the same subscription replaces its previous nodes.
        let mut nodes = state.nodes.lock().unwrap();
        if added > 0 {
            nodes.retain(|n| n.subscription.as_deref() != Some(url.as_str()));
        }
        nodes.extend(parsed);
    }
    state.save_nodes()?;
    ensure_selection(&state)?;
    Ok(ImportResult { added, errors })
}

#[tauri::command]
pub fn remove_node(id: String, state: State<'_, AppState>) -> Result<(), String> {
    state.nodes.lock().unwrap().retain(|n| n.id != id);
    {
        let mut s = state.settings.lock().unwrap();
        if s.selected_node.as_deref() == Some(id.as_str()) {
            s.selected_node = None;
        }
    }
    state.save_nodes()?;
    state.save_settings()
}

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Settings {
    state.settings.lock().unwrap().clone()
}

#[tauri::command]
pub fn set_settings(settings: Settings, state: State<'_, AppState>) -> Result<(), String> {
    *state.settings.lock().unwrap() = settings;
    state.save_settings()
}

#[tauri::command]
pub fn select_node(id: String, state: State<'_, AppState>) -> Result<(), String> {
    state.settings.lock().unwrap().selected_node = Some(id);
    state.save_settings()
}

#[tauri::command]
pub async fn connect(app: AppHandle) -> Result<CoreStatus, String> {
    engine::start(&app).await
}

#[tauri::command]
pub async fn disconnect(app: AppHandle) -> Result<CoreStatus, String> {
    engine::stop(&app).await
}

#[tauri::command]
pub fn core_status(app: AppHandle) -> CoreStatus {
    engine::status(&app)
}

fn store_latency(state: &AppState, id: &str, ms: Option<u32>) {
    if let Some(n) = state.nodes.lock().unwrap().iter_mut().find(|n| n.id == id) {
        n.latency = ms;
    }
}

/// TCP connect latency of one node (works while disconnected).
#[tauri::command]
pub async fn tcp_ping(id: String, state: State<'_, AppState>) -> Result<Option<u32>, String> {
    let (server, port) = state
        .nodes
        .lock()
        .unwrap()
        .iter()
        .find(|n| n.id == id)
        .map(|n| (n.server.clone(), n.port))
        .ok_or("unknown node")?;
    let ms = engine::tcp_ping(&server, port).await;
    store_latency(&state, &id, ms);
    Ok(ms)
}

#[tauri::command]
pub async fn tcp_ping_all(state: State<'_, AppState>) -> Result<(), String> {
    let targets: Vec<(String, String, u16)> = state
        .nodes
        .lock()
        .unwrap()
        .iter()
        .map(|n| (n.id.clone(), n.server.clone(), n.port))
        .collect();
    let results = futures_join(targets).await;
    for (id, ms) in results {
        store_latency(&state, &id, ms);
    }
    state.save_nodes()
}

async fn futures_join(targets: Vec<(String, String, u16)>) -> Vec<(String, Option<u32>)> {
    let handles: Vec<_> = targets
        .into_iter()
        .map(|(id, server, port)| tauri::async_runtime::spawn(async move { (id, engine::tcp_ping(&server, port).await) }))
        .collect();
    let mut out = vec![];
    for h in handles {
        if let Ok(r) = h.await {
            out.push(r);
        }
    }
    out
}

/// Real HTTP round-trip through the active tunnel; requires being connected.
#[tauri::command]
pub async fn http_ping(state: State<'_, AppState>) -> Result<u32, String> {
    if state.active_node.lock().unwrap().is_none() {
        return Err("connect first to measure real HTTP latency".into());
    }
    let (port, url) = {
        let ping_url = state.settings.lock().unwrap().ping_url.clone();
        (state.active_ports.lock().unwrap().0, ping_url)
    };
    let ms = engine::http_ping(port, &url).await?;
    if let Some(id) = state.active_node.lock().unwrap().clone() {
        store_latency(&state, &id, Some(ms));
    }
    Ok(ms)
}
