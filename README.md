<div align="center">

<img src="Logo/with-typo-icon.png" alt="Vanish" width="180" />

# Vanish

**A fast, modern V2Ray / sing-box client for Windows and Android.**<br />
One-tap connect, subscriptions with usage bars, a Game mode for low-lag play, and a clean dark UI in English and Persian.

[![Release](https://github.com/Sobhanafkhami/vanish/actions/workflows/release.yml/badge.svg)](https://github.com/Sobhanafkhami/vanish/actions/workflows/release.yml)
[![Latest release](https://img.shields.io/github/v/release/Sobhanafkhami/vanish?label=download&color=22c55e)](https://github.com/Sobhanafkhami/vanish/releases/latest)

**English** · [فارسی](README.fa.md)

</div>

<p align="center">
  <img src="docs/screenshots/desktop-en.png" alt="Vanish on Windows" width="820" />
</p>

> Screenshots use made-up servers and documentation IP addresses.

## Features

- **Many protocols** — `vless://`, `vmess://`, `trojan://`, `ss://`, `wireguard://`, and raw sing-box outbound JSON (the way to bring in ShadowTLS-chained configs).
- **Subscriptions** — plain or base64 lists, auto-refresh in the background, per-subscription tabs, and usage bars (traffic used / remaining, days left) read from the panel's headers.
- **Three ways in (Windows)** — system proxy, TUN (the whole PC), or manual (just a local port).
- **Routing** — Global, Rule (`.ir` and your own domain/app lists go direct) or Direct, with a DoH resolver of your choice.
- **Game mode** — TUN with a short list of games that use the tunnel while everything else stays direct, keep-alives, full-cone NAT, a live ping / jitter / loss meter and fast failover. See [Game mode](#game-mode).
- **Tools** — TCP ping, *real delay* (a real HTTP request through each server, also while disconnected on Android), "pick the fastest", a speed test and a leak test (IP / DNS / IPv6).
- **Resilience** — auto fallback to the next server when one stops answering, and an optional kill switch.
- **Share and import** — QR codes for servers and subscriptions, one-tap import from the clipboard.
- **Persian (RTL) UI** — a proper right-to-left layout with a Persian font, not a mirrored afterthought.
- **Self-updating** — signed updates on Windows; on Android the app checks GitHub and shows a snackbar when a new APK is out.

<table>
  <tr>
    <td align="center"><img src="docs/screenshots/desktop-game.png" alt="Game mode on desktop" width="400" /><br /><sub>Game mode with a live lag meter</sub></td>
    <td align="center"><img src="docs/screenshots/desktop-fa.png" alt="Persian right-to-left UI" width="400" /><br /><sub>Persian, right-to-left</sub></td>
  </tr>
</table>

## Android

The Android app has the same engine (sing-box as an in-process library behind Android's `VpnService`) and a single-column layout built for thumbs: bottom tabs, a bottom sheet for per-server actions, and full server names.

<table>
  <tr>
    <td align="center"><img src="docs/screenshots/android-home.png" alt="Android: connection" width="250" /><br /><sub>Connection</sub></td>
    <td align="center"><img src="docs/screenshots/android-servers.png" alt="Android: servers" width="250" /><br /><sub>Servers</sub></td>
    <td align="center"><img src="docs/screenshots/android-game.png" alt="Android: game mode" width="250" /><br /><sub>Game mode (per-app)</sub></td>
  </tr>
</table>

## Download

| Platform | Get it |
|---|---|
| Windows | [Latest installer](https://github.com/Sobhanafkhami/vanish/releases/latest) (`Vanish_<version>_x64-setup.exe` or `.msi`). Every push to `main` is built by GitHub Actions and published as a release. |
| Android 7+ (arm64) | `Vanish_<version>_android-arm64.apk` on the [latest release](https://github.com/Sobhanafkhami/vanish/releases/latest). Allow "install unknown apps" for your browser or file manager the first time. |

TUN mode on Windows needs administrator rights. On Android, the first connection shows the system's VPN permission dialog.

## Quick start

1. Open **Servers → Import** and choose **Import from clipboard** after copying a subscription link (or a pile of `vless://` links), or add a subscription URL by hand.
2. Pick a server (or tap **Fastest**) and press the big power button.
3. Optional: turn on **Game mode**, add your games, and Vanish routes only them through the tunnel.

## Game mode

Game mode is built around what a single client can honestly do for lag:

- only the games you list use the tunnel; the rest of your traffic keeps its own, shorter path (per-process rules on Windows, per-app VPN on Android);
- TUN with full-cone NAT, so UDP and peer-to-peer traffic behave;
- short TCP keep-alives and a probe every two seconds that doubles as a keep-alive for the tunnel and feeds the **ping / jitter / loss** readout;
- if the server stops answering, Vanish moves to the next-best one in seconds instead of half a minute.

It is **not** a multi-path "route optimizer" like commercial game VPNs: those run their own server networks and duplicate packets across them, which a client for your own servers cannot do.

## Build from source

Requirements: Node 20+, a Rust toolchain and the [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/) (on Windows: MSVC Build Tools).

```bash
npm install
npm run fetch-core        # downloads the official sing-box into src-tauri/binaries (bundled into the app)
npm run tauri dev         # a second, separate "dev" instance can run next to an installed Vanish
```

```bash
npm run build:app         # NSIS + MSI installers in src-tauri/target/release/bundle/
```

Core lookup order: the `core_path` setting, the app data `bin/` folder, bundled resources, next to the executable, then `PATH`. If the configured ports are taken (for example by Nekoray), Vanish picks free ones. Data lives in `%APPDATA%\app.vanish.client` (`nodes.json`, `settings.json`). Only one instance of the installed app runs at a time.

**Android:** see [docs/android.md](docs/android.md) for the toolchain, building `libbox.aar` with `scripts/build-libbox.sh`, and the signing setup.

## Releases

`.github/workflows/release.yml` runs on every push to `main`:

1. **`release`** builds the signed NSIS and MSI installers on Windows, stamps the version `<major>.<minor>.<run number>` (from `package.json`) and creates the GitHub release `v<version>` together with `latest.json` for the in-app updater.
2. **`android`** builds `libbox` (cached) and a signed release APK and attaches `Vanish_<version>_android-arm64.apk` to that same release.

Repository secrets:

| Secret | Used for |
|---|---|
| `TAURI_SIGNING_PRIVATE_KEY` (+ `_PASSWORD` if it has one) | signing the Windows updater packages |
| `ANDROID_KEYSTORE_BASE64`, `ANDROID_KEYSTORE_PASSWORD`, `ANDROID_KEY_ALIAS`, `ANDROID_KEY_PASSWORD` | signing the Android APK |

Keep both keys backed up and out of the repository: without the Windows key existing installs cannot be updated, and an Android update only installs over an app signed with the same keystore. To start a new minor line, bump `version` in `package.json` (for example `0.2.0`).

## How it works

```
Vue 3 + Tailwind UI ──invoke / events──▶ Rust (Tauri v2)
                                          ├─ nodes, subscriptions, usage, settings
                                          ├─ builds the sing-box config (routing, DNS, TUN, game mode)
                                          └─ supervises the core
Windows:  sing-box child process  +  system proxy / TUN
Android:  libbox (in-process)     +  VpnService (Kotlin plugin)
```

The UI talks to the core only through the Clash API on `127.0.0.1`, so traffic graphs, delay tests and the exit-IP card work the same on both platforms.

## Status

Vanish is young. Windows is the primary target; the Android app has been tested on one physical phone (Android 12) and builds for arm64 only. Not done yet: an xray-core engine (the switch exists but returns an error) and macOS system proxy. Reports and pull requests are welcome.

## Credits

Built on [sing-box](https://github.com/SagerNet/sing-box), [Tauri](https://tauri.app), [Vue](https://vuejs.org) and [Lucide](https://lucide.dev) icons. The Persian UI uses [Vazirmatn](https://github.com/rastikerdar/vazirmatn).
