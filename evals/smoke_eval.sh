#!/usr/bin/env bash
set -uo pipefail

# Run the Gaggle desktop smoke test against a built binary.
# Usage: smoke_eval.sh <path-to-gaggle-desktop-binary>
# Prints PASS or FAIL and exits 0/1 respectively.

if [[ $# -ne 1 ]]; then
  echo "Usage: $0 <path-to-gaggle-desktop-binary>" >&2
  exit 2
fi

BINARY="$1"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
SMOKE="$REPO_ROOT/.github/scripts/smoke-test-server.sh"

if [[ ! -x "$SMOKE" ]]; then
  echo "FAIL"
  echo "smoke script missing or not executable: $SMOKE" >&2
  exit 1
fi

if [[ ! -x "$BINARY" ]]; then
  echo "FAIL"
  echo "binary missing or not executable: $BINARY" >&2
  exit 1
fi

if "$SMOKE" "$BINARY" >/dev/null 2>&1; then
  echo "PASS"
  exit 0
else
  echo "FAIL"
  exit 1
fi
