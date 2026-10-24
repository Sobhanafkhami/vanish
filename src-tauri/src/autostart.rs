//! Start-with-OS registration. Launches minimized to the tray via `--minimized`.

#[cfg(windows)]
pub fn set(enabled: bool) -> Result<(), String> {
    use winreg::{enums::*, RegKey};
    const KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
    let (key, _) = RegKey::predef(HKEY_CURRENT_USER).create_subkey(KEY).map_err(|e| e.to_string())?;
    if enabled {
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        key.set_value("Vanish", &format!("\"{}\" --minimized", exe.display())).map_err(|e| e.to_string())
    } else {
        match key.delete_value("Vanish") {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e.to_string()),
            _ => Ok(()),
        }
    }
}

#[cfg(not(windows))]
pub fn set(_enabled: bool) -> Result<(), String> {
    Err("start with system is only implemented for Windows".into())
}
