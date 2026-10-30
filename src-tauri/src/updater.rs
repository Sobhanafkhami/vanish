//! In-app updates from GitHub releases (signed `latest.json`, see scripts/make-latest-json.mjs).

use std::time::Duration;

use serde::Serialize;
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

// Android: the GitHub release carries the signed APK next to the desktop installers. The newest tag is compared
// with the running version, the APK is downloaded to the cache folder and handed to the system installer, which only
// accepts it if it is signed with the same key as the installed app.
#[cfg(target_os = "android")]
const REPO: &str = "Sobhanafkhami/vanish";

#[cfg(target_os = "android")]
static APK_URL: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);

#[cfg(target_os = "android")]
fn version_parts(v: &str) -> Vec<u64> {
    v.trim_start_matches('v').split(|c: char| !c.is_ascii_digit()).filter(|p| !p.is_empty()).map(|p| p.parse().unwrap_or(0)).collect()
}

#[cfg(target_os = "android")]
fn http() -> Result<reqwest::Client, String> {
    reqwest::Client::builder().user_agent("Vanish-updater").timeout(Duration::from_secs(20)).build().map_err(|e| e.to_string())
}

#[cfg(target_os = "android")]
pub async fn check(app: &AppHandle) -> Result<Option<UpdateInfo>, String> {
    let release: serde_json::Value = http()?
        .get(format!("https://api.github.com/repos/{REPO}/releases/latest"))
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .and_then(|r| r.error_for_status())
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;
    let tag = release["tag_name"].as_str().unwrap_or_default().trim_start_matches('v').to_string();
    if tag.is_empty() || version_parts(&tag) <= version_parts(&app.package_info().version.to_string()) {
        return Ok(None);
    }
    // The release is created before the APK is attached; until it is there there is nothing to offer.
    let apk = release["assets"]
        .as_array()
        .and_then(|a| a.iter().find(|x| x["name"].as_str().is_some_and(|n| n.ends_with("android-arm64.apk"))))
        .and_then(|x| x["browser_download_url"].as_str());
    let Some(url) = apk else { return Ok(None) };
    *APK_URL.lock().unwrap() = Some(url.to_string());
    Ok(Some(UpdateInfo { version: tag, notes: release["body"].as_str().map(str::to_string) }))
}

#[cfg(target_os = "android")]
pub async fn install(app: &AppHandle) -> Result<(), String> {
    use futures_util::StreamExt;
    use std::io::Write;
    use tauri::Manager;

    let known = APK_URL.lock().unwrap().clone();
    let url = match known {
        Some(u) => u,
        None => {
            check(app).await?;
            APK_URL.lock().unwrap().clone().ok_or("already up to date")?
        }
    };
    let dir = app.path().app_cache_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join("update.apk");

    let resp = http()?.get(&url).timeout(Duration::from_secs(900)).send().await.and_then(|r| r.error_for_status()).map_err(|e| e.to_string())?;
    let total = resp.content_length().unwrap_or(0);
    let mut file = std::fs::File::create(&path).map_err(|e| e.to_string())?;
    let (mut done, mut stream) = (0u64, resp.bytes_stream());
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| e.to_string())?;
        file.write_all(&chunk).map_err(|e| e.to_string())?;
        done += chunk.len() as u64;
        let _ = app.emit("update-progress", (done, total));
    }
    drop(file);
    crate::vpn::install_apk(app, &path).await
}

/// Quiet check shortly after launch (the UI then shows a snackbar), then twice a day.
#[cfg(target_os = "android")]
pub async fn check_loop(app: AppHandle) {
    tokio::time::sleep(Duration::from_secs(6)).await;
    loop {
        if let Ok(Some(info)) = check(&app).await {
            let _ = app.emit("update-available", info);
        }
        tokio::time::sleep(Duration::from_secs(12 * 3600)).await;
    }
}
