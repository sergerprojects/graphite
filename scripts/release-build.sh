#!/usr/bin/env bash
# Maintainer script: build the release .app with personal paths removed, check it
# for leaks, and write dist/Graphite-macos-arm64.zip (the name install-release.sh expects).
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
export RUSTFLAGS="--remap-path-prefix=$HOME=/home/build"
export CFLAGS="-ffile-prefix-map=$HOME=/home/build" CXXFLAGS="-ffile-prefix-map=$HOME=/home/build"
export CMAKE_DISABLE_FIND_PACKAGE_OpenSSL=ON CARGO_PROFILE_RELEASE_STRIP=none
security find-identity -p codesigning | grep -q "Graphite Local" && export APPLE_SIGNING_IDENTITY="Graphite Local"

# Updates are signed so installed copies can verify them. The key lives outside the repo.
KEY="${TAURI_SIGNING_PRIVATE_KEY_PATH:-$HOME/.graphite/updater.key}"
[ -f "$KEY" ] || { echo "BUILD FAILED: updater signing key not found at $KEY"; exit 1; }
export TAURI_SIGNING_PRIVATE_KEY="$(cat "$KEY")" TAURI_SIGNING_PRIVATE_KEY_PASSWORD="${TAURI_SIGNING_PRIVATE_KEY_PASSWORD:-}"

(cd llama-helper && cargo build --release --features metal)
cp target/release/llama-helper frontend/src-tauri/binaries/llama-helper-aarch64-apple-darwin
APP=target/release/bundle/macos/Graphite.app
rm -rf "$APP"   # never package a stale bundle from an earlier build
(cd frontend && npx pnpm@10 run tauri:build -- --bundles app) || echo "(updater key error at the end is expected)"
[ -d "$APP" ] || { echo "BUILD FAILED: no app bundle was produced"; exit 1; }
WANT=$(grep -m1 '"version"' frontend/package.json | sed 's/[^0-9.]//g')
GOT=$(defaults read "$PWD/$APP/Contents/Info.plist" CFBundleShortVersionString)
[ "$WANT" = "$GOT" ] || { echo "BUILD FAILED: bundle is $GOT, expected $WANT"; exit 1; }
for f in "$APP"/Contents/MacOS/*; do
  n=$(strings -a "$f" | grep -c "$(whoami)" || true)
  [ "$n" = "0" ] || { echo "LEAK: $f has $n strings containing '$(whoami)'"; exit 1; }
done
codesign --verify --deep --strict "$APP"
# The in-app updater downloads this (a tarball of the app and its signature), not the zip.
[ -f "$APP.tar.gz" ] && [ -f "$APP.tar.gz.sig" ] || { echo "BUILD FAILED: no signed updater package (is the signing key right?)"; exit 1; }
mkdir -p dist && rm -f dist/Graphite-macos-arm64.zip dist/Graphite-macos-arm64.app.tar.gz dist/Graphite-macos-arm64.app.tar.gz.sig
cp "$APP.tar.gz" dist/Graphite-macos-arm64.app.tar.gz
cp "$APP.tar.gz.sig" dist/Graphite-macos-arm64.app.tar.gz.sig
ditto -c -k --sequesterRsrc --keepParent "$APP" dist/Graphite-macos-arm64.zip
shasum -a 256 dist/Graphite-macos-arm64.zip
echo "OK: $(defaults read "$PWD/$APP/Contents/Info.plist" CFBundleShortVersionString) -> dist/Graphite-macos-arm64.zip"
