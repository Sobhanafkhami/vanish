# Android build

The Android app reuses the Vue UI and the Rust engine. The VPN itself is sing-box's `libbox` running inside a
`VpnService` (`src-tauri/gen/android/app/src/main/java/app/vanish/client/`). Rust talks to it through the
`VpnPlugin` Tauri plugin (`src-tauri/src/vpn.rs`); the core stays reachable over the Clash API on 127.0.0.1, so
traffic stats and latency work as on desktop.

## One-time setup

- JDK 17, Android SDK (platform 36, build-tools 36.0.0), NDK r27c, `rustup target add aarch64-linux-android`
- Windows: turn on Developer Mode (Tauri symlinks the `.so` into the Gradle project)
- `libbox.aar` in `src-tauri/gen/android/app/libs/` (git-ignored, ~24 MB)

## Building libbox.aar

```bash
scripts/build-libbox.sh      # needs Go 1.25+, JDK 17, Android SDK + NDK r27 (licenses accepted)
```

The script clones sing-box (`SING_BOX_TAG`, default v1.14.3), applies two small edits to its build tool (drops the
`with_naive_outbound` tag, whose `libcronet.a` needs the NDK r28 linker, and skips the SDK 21 variant), builds the arm64
library with gomobile and copies it to `src-tauri/gen/android/app/libs/libbox.aar`.

## APK

```bash
npx tauri android build --debug --target aarch64 --apk
```

## Known gaps

- "Real delay" runs through the connected core, or through a short-lived headless core when disconnected.
- The Core log view tails the file libbox writes (`core.log` in the app data folder).
- Only arm64 has a libbox build; other ABIs would crash at start.
- Tested on a physical Android 12 phone (Samsung A24); no emulator build.

## Releases and updates

The `android` job in `.github/workflows/release.yml` runs after the desktop release is created: it builds libbox
(cached), builds a **signed release APK** and uploads `Vanish_<version>_android-arm64.apk` to the same GitHub release.

One-time setup. Create a keystore and **back it up**: an update only installs over an app signed with the same key.

```bash
keytool -genkeypair -v -keystore vanish.jks -alias vanish -keyalg RSA -keysize 2048 -validity 10000
base64 -w0 vanish.jks     # value for ANDROID_KEYSTORE_BASE64
```

Add these repository secrets: `ANDROID_KEYSTORE_BASE64`, `ANDROID_KEYSTORE_PASSWORD`, `ANDROID_KEY_ALIAS`,
`ANDROID_KEY_PASSWORD`. Never commit the keystore.

In the app, "Check for updates" (Settings) and a snackbar shown a few seconds after launch use the GitHub API to find
a newer release that has an `android-arm64.apk` asset, download it and open the system installer.
