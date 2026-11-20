#!/usr/bin/env bash
# Maintainer script: write dist/latest.json, the file installed copies of Graphite read to find
# updates. Run after scripts/release-build.sh. Upload it to the release together with the zip,
# dist/Graphite-macos-arm64.app.tar.gz and its .sig:
#   scripts/make-latest-json.sh [notes]
# The Windows entry is taken from the windows-beta release when its signature is there.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
REPO="sergerprojects/graphite"
VERSION=$(grep -m1 '"version"' frontend/package.json | sed 's/[^0-9.]//g')
NOTES="${1:-Graphite $VERSION}"
SIG_FILE=dist/Graphite-macos-arm64.app.tar.gz.sig
[ -f "$SIG_FILE" ] || { echo "No $SIG_FILE: run scripts/release-build.sh first."; exit 1; }
WIN_SIG=$(curl -fsSL "https://github.com/$REPO/releases/download/windows-beta/Graphite-windows-x64-setup.exe.sig" 2>/dev/null || true)
VERSION="$VERSION" NOTES="$NOTES" REPO="$REPO" SIG="$(cat "$SIG_FILE")" WIN_SIG="$WIN_SIG" python3 - <<'PY' > dist/latest.json
import json, os, datetime
repo, ver = os.environ["REPO"], os.environ["VERSION"]
platforms = {
    "darwin-aarch64": {
        "signature": os.environ["SIG"].strip(),
        "url": f"https://github.com/{repo}/releases/download/v{ver}/Graphite-macos-arm64.app.tar.gz",
    }
}
if os.environ["WIN_SIG"].strip():
    platforms["windows-x86_64"] = {
        "signature": os.environ["WIN_SIG"].strip(),
        "url": f"https://github.com/{repo}/releases/download/windows-beta/Graphite-windows-x64-setup.exe",
    }
print(json.dumps({
    "version": ver,
    "notes": os.environ["NOTES"],
    "pub_date": datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
    "platforms": platforms,
}, indent=2))
PY
echo "wrote dist/latest.json (version $VERSION, platforms: $(python3 -c 'import json;print(", ".join(json.load(open("dist/latest.json"))["platforms"]))'))"
