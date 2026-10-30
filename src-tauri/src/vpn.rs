//! Android: the VPN itself lives in Kotlin (`VanishVpnService`, which hosts sing-box's libbox). This module is the
//! bridge to its Tauri plugin, `VpnPlugin`. The core runs in this same process, so the Clash API and the mixed
//! inbound stay reachable on 127.0.0.1 and the rest of the engine works unchanged.

use std::path::Path;

use serde::Deserialize;
use serde_json::json;
use tauri::{
    plugin::{Builder, PluginHandle, TauriPlugin},
    AppHandle, Manager, Wry,
};

pub struct Vpn(PluginHandle<Wry>);

pub fn init() -> TauriPlugin<Wry> {
    Builder::new("vpn")
        .setup(|app, api| {
            let handle = api.register_android_plugin("app.vanish.client", "VpnPlugin")?;
            app.manage(Vpn(handle));
            Ok(())
        })
        .build()
}

#[derive(Deserialize)]
struct Granted {
    granted: bool,
}

#[derive(Deserialize)]
pub struct Status {
    pub running: bool,
    pub error: Option<String>,
}

#[derive(Deserialize)]
struct Apps {
    apps: Vec<AppEntry>,
}

#[derive(Deserialize)]
pub struct AppEntry {
    pub name: String,
    pub title: String,
}

fn handle(app: &AppHandle) -> tauri::State<'_, Vpn> {
    app.state::<Vpn>()
}

/// Asks Android for the VPN permission (shows the system dialog the first time).
pub async fn prepare(app: &AppHandle) -> Result<bool, String> {
    let r: Granted = handle(app).0.run_mobile_plugin_async("prepare", json!({})).await.map_err(|e| e.to_string())?;
    Ok(r.granted)
}

/// `probe`: run the core headless (no VPN interface, no permission) just to time servers.
pub async fn start(app: &AppHandle, config: &Path, probe: bool) -> Result<(), String> {
    handle(app)
        .0
        .run_mobile_plugin_async::<serde_json::Value>("start", json!({ "config": config.to_string_lossy(), "probe": probe }))
        .await
        .map(|_| ())
        .map_err(|e| e.to_string())
}

pub async fn stop(app: &AppHandle) -> Result<(), String> {
    handle(app).0.run_mobile_plugin_async::<serde_json::Value>("stop", json!({})).await.map(|_| ()).map_err(|e| e.to_string())
}

pub async fn status(app: &AppHandle) -> Result<Status, String> {
    handle(app).0.run_mobile_plugin_async("status", json!({})).await.map_err(|e| e.to_string())
}

/// Launchable apps installed on the phone: `name` is the package id, `title` the label.
pub async fn list_apps(app: &AppHandle) -> Result<Vec<AppEntry>, String> {
    let r: Apps = handle(app).0.run_mobile_plugin_async("listApps", json!({})).await.map_err(|e| e.to_string())?;
    Ok(r.apps)
}
