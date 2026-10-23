# Vanish

A lightweight, modern desktop client for **sing-box** / V2Ray connections — built with Tauri v2 (Rust) and Vue 3 + Tailwind, styled as a dark cyber-glass HUD.

## Status (MVP scaffold)

- Frameless glass window with custom titlebar, minimize-to-tray (closing the window hides it; quit from the tray menu).
- Import subscriptions (plain or base64) and single links: `vless://`, `vmess://`, `trojan://`, `ss://`, `wireguard://`; raw sing-box outbound JSON (the way to bring in ShadowTLS-chained configs).
- sing-box child-process supervisor with dynamically generated JSON config (mixed inbound, optional TUN).
- System proxy set/restore on Windows (registry + WinINet refresh) and GNOME/Linux (`gsettings`).
- Live traffic (via the sing-box Clash API), TCP ping for any node, real HTTP ping through the active tunnel.

Not done yet: xray-core fallback (engine switch exists but returns an error), macOS system proxy, subscription auto-update, tests beyond the link-parser unit tests.

## Run

Requirements: Node 20+, Rust toolchain, [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/) (on Windows: MSVC Build Tools).

```bash
npm install
npm run fetch-core   # downloads the official sing-box 1.14.x into src-tauri/binaries (bundled into the app)
npm run tauri dev
```

## Build the Windows .exe / installers

```bash
npm run build:app
```

Outputs: `src-tauri/target/release/vanish.exe` (portable; needs sing-box in `%APPDATA%pp.vanish.clientin\` or next to it), plus NSIS and MSI installers in `src-tauri/target/release/bundle/` which ship sing-box inside.

Core lookup order: `core_path` setting, app-data `bin/`, bundled resources, next to the exe, `PATH`. If the configured ports are taken (e.g. by Nekoray) Vanish picks free ones automatically. TUN mode needs administrator/root privileges. Data lives in `%APPDATA%pp.vanish.client` (`nodes.json`, `settings.json`); only one instance can run.
