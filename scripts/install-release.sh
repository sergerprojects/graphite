#!/usr/bin/env bash
# One-line installer for Graphite (Apple Silicon Macs).
#   curl -fsSL https://raw.githubusercontent.com/sergerprojects/graphite/main/scripts/install-release.sh | bash
# Downloads the latest release with curl, so macOS doesn't flag the app as
# "downloaded from the internet" and block the first launch.
set -euo pipefail

URL="https://github.com/sergerprojects/graphite/releases/latest/download/Graphite-macos-arm64.zip"
APP="/Applications/Graphite.app"

[ "$(uname -s)" = "Darwin" ] && [ "$(uname -m)" = "arm64" ] || { echo "Graphite needs a Mac with Apple Silicon (M1 or newer)."; exit 1; }

# Needs macOS 14.4+ (system-audio capture).
MACOS="$(sw_vers -productVersion)"
if [ "$(printf '%s\n14.4\n' "$MACOS" | sort -V | head -n1)" != "14.4" ]; then
  echo "Graphite needs macOS 14.4 or newer (this Mac has $MACOS)."; exit 1
fi

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

echo "Downloading Graphite (about 50 MB)..."
curl -fL --progress-bar -o "$TMP/Graphite.zip" "$URL"

echo "Installing..."
osascript -e 'tell application id "com.graphite.ai" to quit' >/dev/null 2>&1 || true
sleep 1
ditto -x -k "$TMP/Graphite.zip" "$TMP/out"
[ -d "$TMP/out/Graphite.app" ] || { echo "Download looks wrong: no Graphite.app inside."; exit 1; }
[ -d "$APP" ] && rm -rf "$APP"
ditto "$TMP/out/Graphite.app" "$APP"
xattr -cr "$APP"

echo "Done. Opening Graphite..."
echo "Tip: summaries and chat use Claude Code (claude.com/claude-code). Install it and run 'claude' once to sign in."
open "$APP"
