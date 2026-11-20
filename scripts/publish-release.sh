#!/usr/bin/env bash
# Maintainer script: publish the current version to GitHub, for Mac and Windows.
#   scripts/publish-release.sh <notes-file>
# Run after scripts/release-build.sh, and after the Windows build for this commit has passed
# (it uploads Graphite_<version>_x64-setup.exe to the windows-beta release).
# The update signing key stays on this Mac: the Windows installer is signed here.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
NOTES="${1:?usage: scripts/publish-release.sh <notes-file>}"
REPO="sergerprojects/graphite"
GHG="$HOME/.local/bin/gh-graphite"   # gh as the account that owns the repository
VERSION=$(grep -m1 '"version"' frontend/package.json | sed 's/[^0-9.]//g')
KEY="${TAURI_SIGNING_PRIVATE_KEY_PATH:-$HOME/.graphite/updater.key}"
[ -f "$KEY" ] || { echo "No signing key at $KEY"; exit 1; }
for f in dist/Graphite-macos-arm64.zip dist/Graphite-macos-arm64.app.tar.gz dist/Graphite-macos-arm64.app.tar.gz.sig; do
  [ -f "$f" ] || { echo "Missing $f: run scripts/release-build.sh"; exit 1; }
done
GOT=$(defaults read "$PWD/target/release/bundle/macos/Graphite.app/Contents/Info.plist" CFBundleShortVersionString)
[ "$GOT" = "$VERSION" ] || { echo "The built app is $GOT but the code says $VERSION: rebuild first."; exit 1; }

# The Windows installer built from this exact version.
WIN="dist/win/Graphite-windows-x64-setup.exe"
mkdir -p dist/win && rm -f dist/win/*
"$GHG" release download windows-beta --repo "$REPO" --pattern "Graphite_${VERSION}_x64-setup.exe" --dir dist/win \
  || { echo "No Windows installer for $VERSION on windows-beta: wait for the Windows build to pass."; exit 1; }
mv "dist/win/Graphite_${VERSION}_x64-setup.exe" "$WIN"
# Sign it here so the private key never leaves this Mac.
(cd frontend && TAURI_SIGNING_PRIVATE_KEY="$(cat "$KEY")" TAURI_SIGNING_PRIVATE_KEY_PASSWORD="" npx tauri signer sign "../$WIN" >/dev/null)
[ -f "$WIN.sig" ] || { echo "Signing the Windows installer failed."; exit 1; }

# The file installed copies read to find updates (both platforms, this release's own assets).
VERSION="$VERSION" REPO="$REPO" NOTES_FILE="$NOTES" MAC_SIG="$(cat dist/Graphite-macos-arm64.app.tar.gz.sig)" WIN_SIG="$(cat "$WIN.sig")" python3 - <<'PY' > dist/latest.json
import json, os, datetime
repo, ver = os.environ["REPO"], os.environ["VERSION"]
base = f"https://github.com/{repo}/releases/download/v{ver}"
print(json.dumps({
    "version": ver,
    "notes": open(os.environ["NOTES_FILE"]).read().split("\n## ")[0].strip()[:600],
    "pub_date": datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
    "platforms": {
        "darwin-aarch64": {"signature": os.environ["MAC_SIG"].strip(), "url": f"{base}/Graphite-macos-arm64.app.tar.gz"},
        "windows-x86_64": {"signature": os.environ["WIN_SIG"].strip(), "url": f"{base}/Graphite-windows-x64-setup.exe"},
    },
}, indent=2))
PY

SHA=$(shasum -a 256 dist/Graphite-macos-arm64.zip | cut -d' ' -f1)
sed "s/@@SHA@@/$SHA/" "$NOTES" > dist/release-notes.md
ASSETS=(dist/Graphite-macos-arm64.zip dist/Graphite-macos-arm64.app.tar.gz dist/Graphite-macos-arm64.app.tar.gz.sig "$WIN" "$WIN.sig" dist/latest.json)
if "$GHG" release view "v$VERSION" --repo "$REPO" >/dev/null 2>&1; then
  "$GHG" release edit "v$VERSION" --repo "$REPO" --title "Graphite $VERSION" --notes-file dist/release-notes.md --latest
  "$GHG" release upload "v$VERSION" --repo "$REPO" "${ASSETS[@]}" --clobber
else
  "$GHG" release create "v$VERSION" --repo "$REPO" --target main --title "Graphite $VERSION" --notes-file dist/release-notes.md --latest "${ASSETS[@]}"
fi
echo "Published v$VERSION: https://github.com/$REPO/releases/tag/v$VERSION"
