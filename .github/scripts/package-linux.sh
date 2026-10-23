#!/usr/bin/env bash
set -euo pipefail

# Assemble a portable Linux application bundle. The Rust server serves the
# embedded web UI; no external frontend runtime is required.

usage() {
  cat <<'USAGE'
Usage: package-linux.sh --target <target> --version <version> --binary-dir <dir> --output-dir <dir> [options]

Options:
  --target <triple>       x86_64-unknown-linux-gnu or aarch64-unknown-linux-gnu
  --version <version>     Release version, for example 0.1.0
  --binary-dir <dir>      Directory containing release `gaggle` and `gaggle-desktop`
  --output-dir <dir>      Destination directory for the archive
  --commit <sha>          Git commit recorded in package metadata (default: unknown)
  --ref <ref>             Git ref recorded in package metadata (default: unknown)
  -h, --help              Show this help
USAGE
}

TARGET=""
VERSION=""
BINARY_DIR=""
OUTPUT_DIR=""
COMMIT="unknown"
REF="unknown"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --target)
      TARGET="${2:?--target requires a value}"
      shift 2
      ;;
    --version)
      VERSION="${2:?--version requires a value}"
      shift 2
      ;;
    --binary-dir)
      BINARY_DIR="${2:?--binary-dir requires a value}"
      shift 2
      ;;
    --output-dir)
      OUTPUT_DIR="${2:?--output-dir requires a value}"
      shift 2
      ;;
    --commit)
      COMMIT="${2:?--commit requires a value}"
      shift 2
      ;;
    --ref)
      REF="${2:?--ref requires a value}"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown argument: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

if [[ -z "$TARGET" || -z "$VERSION" || -z "$BINARY_DIR" || -z "$OUTPUT_DIR" ]]; then
  usage >&2
  exit 2
fi

case "$TARGET" in
  x86_64-unknown-linux-gnu)
    ARCH_LABEL="x86_64"
    ;;
  aarch64-unknown-linux-gnu)
    ARCH_LABEL="aarch64"
    ;;
  *)
    echo "Unsupported Linux target: $TARGET" >&2
    exit 2
    ;;
esac

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
STAGE_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/gaggle-linux-package.XXXXXX")"
trap 'rm -rf "$STAGE_ROOT"' EXIT

PACKAGE_ROOT="$STAGE_ROOT/Gaggle-linux-$ARCH_LABEL"
ARCHIVE="$OUTPUT_DIR/Gaggle-linux-$ARCH_LABEL.tar.gz"

for binary in "$BINARY_DIR/gaggle" "$BINARY_DIR/gaggle-desktop"; do
  if [[ ! -x "$binary" ]]; then
    echo "Required binary is missing or not executable: $binary" >&2
    exit 1
  fi
done

mkdir -p "$OUTPUT_DIR"
rm -f "$ARCHIVE"
mkdir -p \
  "$PACKAGE_ROOT/bin" \
  "$PACKAGE_ROOT/share/applications" \
  "$PACKAGE_ROOT/share/gaggle" \
  "$PACKAGE_ROOT/share/icons/hicolor/scalable/apps"

install -m 755 "$BINARY_DIR/gaggle" "$PACKAGE_ROOT/bin/gaggle"
install -m 755 "$BINARY_DIR/gaggle-desktop" "$PACKAGE_ROOT/bin/gaggle-desktop"
install -m 644 "$ROOT_DIR/docs/assets/gaggle-logo.svg" "$PACKAGE_ROOT/share/gaggle/gaggle-logo.svg"
install -m 644 "$ROOT_DIR/docs/assets/gaggle-logo.svg" "$PACKAGE_ROOT/share/icons/hicolor/scalable/apps/gaggle.svg"
install -m 644 "$ROOT_DIR/LICENSE" "$PACKAGE_ROOT/LICENSE"
install -m 644 "$ROOT_DIR/README.md" "$PACKAGE_ROOT/README.md"

cat > "$PACKAGE_ROOT/share/applications/gaggle.desktop" <<'DESKTOP'
[Desktop Entry]
Type=Application
Name=Gaggle
Comment=Local-first AI agent workspace
Exec=gaggle-desktop
Icon=gaggle
Terminal=false
Categories=Development;Utility;
Keywords=AI;Agent;Development;
StartupNotify=true
DESKTOP

cat > "$PACKAGE_ROOT/install.sh" <<'INSTALL'
#!/bin/sh
set -eu

SOURCE_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
BIN_DIR="${HOME}/.local/bin"
DATA_DIR="${XDG_DATA_HOME:-${HOME}/.local/share}"

mkdir -p "$BIN_DIR" "$DATA_DIR/applications" "$DATA_DIR/icons/hicolor/scalable/apps"

install -m 755 "$SOURCE_DIR/bin/gaggle" "$BIN_DIR/gaggle"
install -m 755 "$SOURCE_DIR/bin/gaggle-desktop" "$BIN_DIR/gaggle-desktop"
install -m 644 "$SOURCE_DIR/share/applications/gaggle.desktop" "$DATA_DIR/applications/gaggle.desktop"
install -m 644 "$SOURCE_DIR/share/icons/hicolor/scalable/apps/gaggle.svg" "$DATA_DIR/icons/hicolor/scalable/apps/gaggle.svg"

if command -v update-desktop-database >/dev/null 2>&1; then
  update-desktop-database "$DATA_DIR/applications" >/dev/null 2>&1 || true
fi

printf '%s\n' \
  "Installed Gaggle into ${BIN_DIR}." \
  "Ensure ${BIN_DIR} is on your PATH, then run: gaggle-desktop" \
  "CLI usage: gaggle --help"
INSTALL
chmod 755 "$PACKAGE_ROOT/install.sh"

cat > "$PACKAGE_ROOT/README.txt" <<README
Gaggle $VERSION for Linux $ARCH_LABEL
Commit: $COMMIT
Source ref: $REF

Run without installing:
  ./bin/gaggle-desktop

Install for the current user (no sudo required):
  ./install.sh

CLI usage:
  ./bin/gaggle --help

The desktop launcher starts the local Gaggle server on 127.0.0.1:4321 and
opens the default browser. The UI is embedded in the server binary; no separate
Node.js or Electron runtime is required.

User data is stored outside this package:
  \${XDG_DATA_HOME:-\$HOME/.local/share}/gaggle/gaggle.db

This archive is a portable Linux build. It is not a .deb, .rpm, or AppImage.
See docs/development/packaging.md in the Gaggle repository for supported
platforms, runtime requirements, and signing status.
README

SOURCE_DATE_EPOCH="${SOURCE_DATE_EPOCH:-$(git -C "$ROOT_DIR" log -1 --format=%ct 2>/dev/null || echo 0)}"
if tar --version 2>/dev/null | grep -q 'GNU tar'; then
  tar \
    --sort=name \
    --mtime="@$SOURCE_DATE_EPOCH" \
    --owner=0 \
    --group=0 \
    --numeric-owner \
    -czf "$ARCHIVE" \
    -C "$STAGE_ROOT" \
    "Gaggle-linux-$ARCH_LABEL"
else
  COPYFILE_DISABLE=1 tar -czf "$ARCHIVE" -C "$STAGE_ROOT" "Gaggle-linux-$ARCH_LABEL"
fi

echo "$ARCHIVE"
