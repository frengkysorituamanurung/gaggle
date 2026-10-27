#!/usr/bin/env bash
# Launch Gaggle in a desktop-like window (Chrome/Edge app mode).
# Requires: gaggle binary built and a Chromium-based browser.

set -euo pipefail

ADDR="${GAGGLE_ADDR:-127.0.0.1:4321}"
DB="${GAGGLE_DB:-$HOME/.local/share/gaggle/gaggle.db}"
mkdir -p "$(dirname "$DB")"

# Start server if not already running
if ! curl -s "http://$ADDR/api/health" >/dev/null 2>&1; then
  gaggle --db "$DB" serve --addr "$ADDR" >/tmp/gaggle-desktop.log 2>&1 &
  SERVER_PID=$!
  for _ in $(seq 1 50); do
    curl -s "http://$ADDR/api/health" >/dev/null 2>&1 && break
    sleep 0.2
  done
else
  SERVER_PID=""
fi

URL="http://$ADDR"

# Try Chrome/Chromium/Edge in app mode
for BROWSER in "Google Chrome" "Chromium" "Microsoft Edge" "Brave Browser"; do
  if [ -d "/Applications/${BROWSER}.app" ]; then
    open -na "${BROWSER}.app" --args --app="$URL" --user-data-dir="/tmp/gaggle-chrome-profile"
    echo "Opened in $BROWSER app mode: $URL"
    exit 0
  fi
done

# Fallback: default browser
open "$URL"
echo "Opened in default browser: $URL"