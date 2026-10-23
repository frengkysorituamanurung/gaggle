#!/usr/bin/env bash
set -euo pipefail

# Regenerate the committed macOS icon from the project SVG logo.
# macOS only: qlmanage, sips, and iconutil are system tools.

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
SOURCE_SVG="$ROOT_DIR/docs/assets/gaggle-logo.svg"
OUTPUT="${1:-$ROOT_DIR/assets/Gaggle.icns}"

for tool in qlmanage sips iconutil; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "Required macOS tool is unavailable: $tool" >&2
    exit 1
  fi
done

if [[ ! -f "$SOURCE_SVG" ]]; then
  echo "Logo does not exist: $SOURCE_SVG" >&2
  exit 1
fi

WORK_DIR="$(mktemp -d "${TMPDIR:-/tmp}/gaggle-icon.XXXXXX")"
trap 'rm -rf "$WORK_DIR"' EXIT

qlmanage -t -s 1024 -o "$WORK_DIR" "$SOURCE_SVG" >/dev/null 2>&1
SOURCE_PNG="$WORK_DIR/gaggle-logo.svg.png"
if [[ ! -f "$SOURCE_PNG" ]]; then
  echo "qlmanage did not produce a PNG thumbnail" >&2
  exit 1
fi

ICONSET="$WORK_DIR/Gaggle.iconset"
mkdir -p "$ICONSET"
for size in 16 32 128 256 512; do
  sips -z "$size" "$size" "$SOURCE_PNG" --out "$ICONSET/icon_${size}x${size}.png" >/dev/null
  doubled=$((size * 2))
  sips -z "$doubled" "$doubled" "$SOURCE_PNG" --out "$ICONSET/icon_${size}x${size}@2x.png" >/dev/null
done

mkdir -p "$(dirname "$OUTPUT")"
iconutil -c icns "$ICONSET" -o "$OUTPUT"
echo "$OUTPUT"
