#!/usr/bin/env bash
set -euo pipefail

# Gaggle's web UI is embedded as a Rust string. Extract the inline scripts and
# ask Node to parse them so a syntax error is caught before packaging.

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
SOURCE_FILE="$ROOT_DIR/crates/gaggle-ui/src/lib.rs"

if [[ ! -f "$SOURCE_FILE" ]]; then
  echo "UI source file does not exist: $SOURCE_FILE" >&2
  exit 1
fi

if ! command -v node >/dev/null 2>&1; then
  echo "node is required to validate the embedded UI scripts" >&2
  exit 1
fi

WORK_DIR="$(mktemp -d "${TMPDIR:-/tmp}/gaggle-ui-check.XXXXXX")"
trap 'rm -rf "$WORK_DIR"' EXIT

SOURCE_FILE="$SOURCE_FILE" \
WORK_DIR="$WORK_DIR" \
python3 - <<'PY'
import os
import pathlib
import re

source = pathlib.Path(os.environ["SOURCE_FILE"]).read_text(encoding="utf-8")
scripts = re.findall(r"<script>(.*?)</script>", source, flags=re.DOTALL)
if not scripts:
    raise SystemExit("No inline <script> blocks found in the UI source")

for index, script in enumerate(scripts, start=1):
    path = pathlib.Path(os.environ["WORK_DIR"]) / f"script-{index}.js"
    path.write_text(script, encoding="utf-8")
    print(path)
PY

for script in "$WORK_DIR"/*.js; do
  node --check "$script"
done

echo "Validated embedded UI scripts"
