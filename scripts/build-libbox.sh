#!/usr/bin/env bash
# Builds libbox.aar (sing-box as an Android library) for arm64 and drops it where the Android project expects it.
# Needs: Go 1.25+, JDK 17, an Android SDK with licenses accepted and an NDK (ANDROID_HOME / ANDROID_NDK_HOME).
# Usage: scripts/build-libbox.sh            (tag defaults to SING_BOX_TAG below)
set -euo pipefail

SING_BOX_TAG="${SING_BOX_TAG:-v1.14.3}"
GOMOBILE_TAG="v0.1.13"
OUT="$(cd "$(dirname "$0")/.." && pwd)/src-tauri/gen/android/app/libs"

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
git clone --depth 1 --branch "$SING_BOX_TAG" https://github.com/SagerNet/sing-box.git "$work/sing-box"
cd "$work/sing-box"

build=cmd/internal/build_libbox/main.go
# naive outbound links Chromium's libcronet, which needs a newer NDK linker than r27; Vanish does not use it.
sed -i 's/"with_naive_outbound", //' "$build"
# minSdk is 24, so the extra SDK-21 variant is not needed.
sed -i 's|^\t// Build legacy variant.*|\tif true {\n\t\treturn\n\t}|' "$build"

go install "github.com/sagernet/gomobile/cmd/gomobile@$GOMOBILE_TAG"
go install "github.com/sagernet/gomobile/cmd/gobind@$GOMOBILE_TAG"
export PATH="$PATH:$(go env GOPATH)/bin"

go run ./cmd/internal/build_libbox -target android -platform android/arm64

mkdir -p "$OUT"
cp libbox.aar "$OUT/libbox.aar"
echo "libbox.aar -> $OUT"
