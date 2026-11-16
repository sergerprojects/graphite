#!/usr/bin/env bash
# Remove Graphite. Your meetings and notes are KEPT unless you pass --everything.
#   bash scripts/uninstall.sh              # remove the app and its login item
#   bash scripts/uninstall.sh --everything # also delete meetings, settings, saved voices
set -euo pipefail
osascript -e 'tell application id "com.graphite.ai" to quit' >/dev/null 2>&1 || true
sleep 1
rm -rf /Applications/Graphite.app
launchctl bootout "gui/$(id -u)/Graphite" >/dev/null 2>&1 || true
rm -f "$HOME/Library/LaunchAgents/Graphite.plist"
if [ "${1:-}" = "--everything" ]; then
  rm -rf "$HOME/Movies/graphite-recordings" "$HOME/.graphite" \
         "$HOME/Library/Application Support/com.graphite.ai" \
         "$HOME/Library/Caches/com.graphite.ai" "$HOME/Library/WebKit/com.graphite.ai"
  echo "Graphite and all of its data are gone."
else
  echo "Graphite is removed. Your meetings are still in ~/Movies/graphite-recordings."
fi
