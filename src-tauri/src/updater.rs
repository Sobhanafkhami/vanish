//! In-app updates from GitHub releases (signed `latest.json`, see scripts/make-latest-json.mjs).

use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Emitter};
use tauri_plugin_updater::UpdaterExt;

use crate::engine;

#[derive(Serialize, Clone)]
pub struct UpdateInfo {
    pub version: String,
    pub notes: Option<String>,
}

pub async fn check(app: &AppHandle) -> Result<Option<UpdateInfo>, String> {
    let update = app.updater().map_err(|e| e.to_string())?.check().await.map_err(|e| e.to_string())?;
    Ok(update.map(|u| UpdateInfo { version: u.version.clone(), notes: u.body.clone() }))
}

/// Downloads and installs the newest release, then restarts. The tunnel is stopped first so the system
/// proxy is restored and the bundled sing-box.exe is not locked while the installer replaces files.
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
pub async fn check_loop(app: AppHandle) {
    tokio::time::sleep(Duration::from_secs(25)).await;
    loop {
        if let Ok(Some(info)) = check(&app).await {
            let _ = app.emit("update-available", info);
        }
        tokio::time::sleep(Duration::from_secs(12 * 3600)).await;
    }
}
