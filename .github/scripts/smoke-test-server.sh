#!/usr/bin/env bash
set -euo pipefail

# Start the packaged desktop launcher against a throwaway database and verify
# that the embedded HTTP server answers /api/health.

if [[ $# -ne 1 ]]; then
  echo "Usage: smoke-test-server.sh <path-to-gaggle-desktop-binary>" >&2
  exit 2
fi

BINARY="$1"
if [[ ! -x "$BINARY" ]]; then
  echo "Binary is missing or not executable: $BINARY" >&2
  exit 1
fi

PORT="${GAGGLE_SMOKE_PORT:-43999}"
WORK_DIR="$(mktemp -d "${TMPDIR:-/tmp}/gaggle-smoke.XXXXXX")"
LOG_FILE="$WORK_DIR/server.log"
PID=""

# shellcheck disable=SC2329
cleanup() {
  if [[ -n "$PID" ]] && kill -0 "$PID" >/dev/null 2>&1; then
    kill -TERM "$PID" >/dev/null 2>&1 || true
    wait "$PID" >/dev/null 2>&1 || true
  fi
  rm -rf "$WORK_DIR"
}
trap cleanup EXIT

"$BINARY" \
  --db "$WORK_DIR/gaggle-smoke.db" \
  --addr "127.0.0.1:$PORT" \
  --no-browser \
  >"$LOG_FILE" 2>&1 &
PID=$!

for _ in $(seq 1 100); do
  if curl -fsS "http://127.0.0.1:$PORT/api/health" 2>/dev/null | grep -q '"status":"ok"'; then
    echo "Smoke test passed: $BINARY"
    exit 0
  fi
  if ! kill -0 "$PID" >/dev/null 2>&1; then
    echo "Server exited before becoming ready:" >&2
    cat "$LOG_FILE" >&2
    exit 1
  fi
  sleep 0.1
done

echo "Server did not become ready within 10 seconds:" >&2
cat "$LOG_FILE" >&2
exit 1
