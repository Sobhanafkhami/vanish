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

Requirements: Node 20+, Rust toolchain, [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/), and a `sing-box` binary (>= 1.11) placed in `<app data dir>/bin/` (e.g. `%APPDATA%\app.vanish.client\bin\sing-box.exe`), on `PATH`, or set via `core_path` in settings.

```bash
npm install
npm run tauri dev
```

TUN mode needs administrator/root privileges. The generated config uses the legacy DNS server format and `wireguard` outbound, which newer sing-box releases deprecate — pin to 1.11.x for now.
