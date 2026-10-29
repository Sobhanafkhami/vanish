//! In-app updates from GitHub releases (signed `latest.json`, see scripts/make-latest-json.mjs).

#[cfg(desktop)]
use std::time::Duration;

use serde::Serialize;
#[cfg(desktop)]
use tauri::Emitter;
use tauri::AppHandle;
#[cfg(desktop)]
use tauri_plugin_updater::UpdaterExt;

#[cfg(desktop)]
use crate::engine;

#[derive(Serialize, Clone)]
pub struct UpdateInfo {
    pub version: String,
    pub notes: Option<String>,
}

#[cfg(desktop)]
pub async fn check(app: &AppHandle) -> Result<Option<UpdateInfo>, String> {
    let update = app.updater().map_err(|e| e.to_string())?.check().await.map_err(|e| e.to_string())?;
    Ok(update.map(|u| UpdateInfo { version: u.version.clone(), notes: u.body.clone() }))
}

/// Downloads and installs the newest release, then restarts. The tunnel is stopped first so the system
/// proxy is restored and the bundled sing-box.exe is not locked while the installer replaces files.
#[cfg(desktop)]
pub async fn install(app: &AppHandle) -> Result<(), String> {
    let update = app
        .updater()
        .map_err(|e| e.to_string())?
        .check()
        .await
        .map_err(|e| e.to_string())?
        .ok_or("already up to date")?;
    let bytes = update
        .download(
            {
                let app = app.clone();
                let mut done = 0u64;
                move |chunk, total| {
                    done += chunk as u64;
                    let _ = app.emit("update-progress", (done, total));
                }
            },
            || {},
        )
        .await
        .map_err(|e| e.to_string())?;
    let _ = engine::stop(app).await;
    update.install(bytes).map_err(|e| e.to_string())?;
    app.restart();
}

/// Quiet background check: once shortly after launch, then twice a day. Failures (offline, no release yet) are ignored.
#[cfg(desktop)]
pub async fn check_loop(app: AppHandle) {
    tokio::time::sleep(Duration::from_secs(25)).await;
    loop {
        if let Ok(Some(info)) = check(&app).await {
            let _ = app.emit("update-available", info);
        }
        tokio::time::sleep(Duration::from_secs(12 * 3600)).await;
    }
}

// Android updates arrive through the store or a new APK, not from inside the app.
#[cfg(not(desktop))]
pub async fn check(_app: &AppHandle) -> Result<Option<UpdateInfo>, String> {
    Ok(None)
}

#[cfg(not(desktop))]
pub async fn install(_app: &AppHandle) -> Result<(), String> {
    Err("updates are installed from the store on this platform".into())
}

#[cfg(not(desktop))]
pub async fn check_loop(_app: AppHandle) {}
