//! OS system-proxy control. Previous settings are remembered on `set` and restored on `clear`.

use std::sync::Mutex;

#[allow(dead_code)]
static PREVIOUS: Mutex<Option<(u32, String)>> = Mutex::new(None);

#[cfg(windows)]
mod imp {
    use super::PREVIOUS;
    use winreg::{enums::*, RegKey};

    const KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Internet Settings";

    fn notify() {
        use windows_sys::Win32::Networking::WinInet::{InternetSetOptionW, INTERNET_OPTION_REFRESH, INTERNET_OPTION_SETTINGS_CHANGED};
        unsafe {
            InternetSetOptionW(std::ptr::null(), INTERNET_OPTION_SETTINGS_CHANGED, std::ptr::null(), 0);
            InternetSetOptionW(std::ptr::null(), INTERNET_OPTION_REFRESH, std::ptr::null(), 0);
        }
    }

    fn open() -> Result<RegKey, String> {
        RegKey::predef(HKEY_CURRENT_USER)
            .create_subkey(KEY)
            .map(|(k, _)| k)
            .map_err(|e| e.to_string())
    }

    pub fn set(addr: &str) -> Result<(), String> {
        let key = open()?;
        let mut prev = PREVIOUS.lock().unwrap();
        if prev.is_none() {
            *prev = Some((
                key.get_value("ProxyEnable").unwrap_or(0u32),
                key.get_value("ProxyServer").unwrap_or_default(),
            ));
        }
        key.set_value("ProxyServer", &addr).map_err(|e| e.to_string())?;
        key.set_value("ProxyOverride", &"localhost;127.*;10.*;172.16.*;192.168.*;<local>").map_err(|e| e.to_string())?;
        key.set_value("ProxyEnable", &1u32).map_err(|e| e.to_string())?;
        notify();
        Ok(())
    }

    pub fn clear() -> Result<(), String> {
        let key = open()?;
        let (enable, server) = PREVIOUS.lock().unwrap().take().unwrap_or((0, String::new()));
        key.set_value("ProxyServer", &server).map_err(|e| e.to_string())?;
        key.set_value("ProxyEnable", &enable).map_err(|e| e.to_string())?;
        notify();
        Ok(())
    }
}

#[cfg(target_os = "linux")]
mod imp {
    use super::PREVIOUS;
    use std::process::Command;

    fn gs(args: &[&str]) -> Result<String, String> {
        let out = Command::new("gsettings").args(args).output().map_err(|e| format!("gsettings: {e}"))?;
        if out.status.success() {
            Ok(String::from_utf8_lossy(&out.stdout).trim().trim_matches('\'').to_string())
        } else {
            Err(String::from_utf8_lossy(&out.stderr).into_owned())
        }
    }

    pub fn set(addr: &str) -> Result<(), String> {
        let (host, port) = addr.rsplit_once(':').ok_or("bad address")?;
        let mut prev = PREVIOUS.lock().unwrap();
        if prev.is_none() {
            *prev = Some((0, gs(&["get", "org.gnome.system.proxy", "mode"]).unwrap_or_else(|_| "none".into())));
        }
        for scheme in ["http", "https", "socks"] {
            let schema = format!("org.gnome.system.proxy.{scheme}");
            gs(&["set", &schema, "host", host])?;
            gs(&["set", &schema, "port", port])?;
        }
        gs(&["set", "org.gnome.system.proxy", "mode", "manual"])?;
        Ok(())
    }

    pub fn clear() -> Result<(), String> {
        let mode = PREVIOUS.lock().unwrap().take().map(|p| p.1).unwrap_or_else(|| "none".into());
        gs(&["set", "org.gnome.system.proxy", "mode", &mode])?;
        Ok(())
    }
}

#[cfg(not(any(windows, target_os = "linux")))]
mod imp {
    pub fn set(_addr: &str) -> Result<(), String> {
        Err("system proxy is only implemented for Windows and Linux (GNOME)".into())
    }
    pub fn clear() -> Result<(), String> {
        Ok(())
    }
}

pub use imp::{clear, set};
