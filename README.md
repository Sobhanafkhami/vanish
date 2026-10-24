# Vanish

[![Release](https://github.com/Sobhanafkhami/vanish/actions/workflows/release.yml/badge.svg)](https://github.com/Sobhanafkhami/vanish/actions/workflows/release.yml)
[![Latest release](https://img.shields.io/github/v/release/Sobhanafkhami/vanish?label=download&color=22c55e)](https://github.com/Sobhanafkhami/vanish/releases/latest)

**[Download the latest Windows installer](https://github.com/Sobhanafkhami/vanish/releases/latest)**: every push to `main` is built by GitHub Actions and published as a release.

A lightweight, modern desktop client for **sing-box** / V2Ray connections — built with Tauri v2 (Rust) and Vue 3 + Tailwind, styled as a dark cyber-glass HUD.

## Status (MVP scaffold)

- Frameless glass window with custom titlebar, minimize-to-tray (closing the window hides it; quit from the tray menu).
- Import subscriptions (plain or base64) and single links: `vless://`, `vmess://`, `trojan://`, `ss://`, `wireguard://`; raw sing-box outbound JSON (the way to bring in ShadowTLS-chained configs).
- sing-box child-process supervisor with dynamically generated JSON config (mixed inbound, optional TUN).
- System proxy set/restore on Windows (registry + WinINet refresh) and GNOME/Linux (`gsettings`).
- Live traffic (via the sing-box Clash API), TCP ping for any node, real HTTP ping through the active tunnel.

Not done yet: xray-core fallback (engine switch exists but returns an error) and macOS system proxy.

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

Outputs: `src-tauri/target/release/vanish.exe` (portable; needs sing-box in `%APPDATA%\app.vanish.client\bin\` or next to it), plus NSIS and MSI installers in `src-tauri/target/release/bundle/` which ship sing-box inside.

Core lookup order: `core_path` setting, app-data `bin/`, bundled resources, next to the exe, `PATH`. If the configured ports are taken (e.g. by Nekoray) Vanish picks free ones automatically. TUN mode needs administrator/root privileges. Data lives in `%APPDATA%\app.vanish.client` (`nodes.json`, `settings.json`); only one instance can run.

## Releases (automatic)

`.github/workflows/release.yml` runs on every push to `main` (docs-only pushes are skipped): it builds the signed NSIS and MSI installers on a Windows runner, stamps the version as `<major>.<minor>` from `package.json` plus the run number (e.g. `0.1.42`), and creates the GitHub release `v0.1.42` with the installers and `latest.json` for the in-app updater. Releases show up in the repository sidebar and on the Releases page.

One-time setup: add the repository secret **`TAURI_SIGNING_PRIVATE_KEY`** (Settings > Secrets and variables > Actions) with the full contents of the updater private key file. If the key has a password, also add `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`. To start a new minor line, bump `version` in `package.json` (e.g. `0.2.0`).

## Releasing manually, with in-app updates

The app checks `https://github.com/Sobhanafkhami/vanish/releases/latest/download/latest.json` and installs signed updates. Update packages must be signed, so the build needs the private key (generated once with `npx tauri signer generate -w ~/.tauri/vanish-updater.key`; the matching public key is in `src-tauri/tauri.conf.json`). **Never commit the private key, and back it up: without it existing installs cannot be updated.**

```powershell
$env:TAURI_SIGNING_PRIVATE_KEY = "$HOME\.tauri\vanish-updater.key"
npm run release            # build:app + writes latest.json next to the NSIS installer
```

Then create a GitHub release tagged `v<version>` and upload `Vanish_<version>_x64-setup.exe`, its `.sig`, and `latest.json` from `src-tauri/target/release/bundle/nsis/`. Bump `version` in `package.json`, `tauri.conf.json` and `Cargo.toml` first.

