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

Use Go 1.25.5 and the sing-box tag in `package.json`/`Cargo` notes (currently v1.14.3):

```bash
go install github.com/sagernet/gomobile/cmd/gomobile@v0.1.13
go install github.com/sagernet/gomobile/cmd/gobind@v0.1.13
go run ./cmd/internal/build_libbox -target android -platform android/arm64
```

Two edits to `cmd/internal/build_libbox/main.go` were needed with NDK r27: drop the `with_naive_outbound` tag
(its `libcronet.a` needs the NDK r28 linker) and skip the legacy SDK 21 variant.

## APK

```bash
npx tauri android build --debug --target aarch64 --apk
```

## Known gaps

- "Real delay" test is desktop-only (it starts a second sing-box process); TCP ping works.
- No core log on Android yet.
- Only arm64 has a libbox build; other ABIs would crash at start.
- Not tried on a device or emulator yet.
