#!/usr/bin/env bash
set -euo pipefail

##############################################################################
# Gaggle Install Script
#
# Downloads the latest Gaggle release for macOS or Linux and installs the
# `gaggle` and `gaggle-desktop` binaries into a user-writable bin directory.
#
# Usage:
#   curl -fsSL https://github.com/frengky/gaggle/releases/latest/download/download_cli.sh | bash
#
# Environment variables:
#   GAGGLE_BIN_DIR   - Install directory (default: $HOME/.local/bin)
#   GAGGLE_VERSION   - Specific tag to install (e.g. v0.1.0). Default: latest
#   GAGGLE_REPO      - GitHub repo (default: frengky/gaggle)
##############################################################################

if ! command -v curl >/dev/null 2>&1; then
  echo "Error: 'curl' is required." >&2
  exit 1
fi
if ! command -v tar >/dev/null 2>&1; then
  echo "Error: 'tar' is required." >&2
  exit 1
fi

REPO="${GAGGLE_REPO:-frengky/gaggle}"
BIN_DIR="${GAGGLE_BIN_DIR:-$HOME/.local/bin}"
mkdir -p "$BIN_DIR"

OS="$(uname -s)"
ARCH="$(uname -m)"
case "$OS" in
  Darwin)
    case "$ARCH" in
      arm64|aarch64) ASSET="Gaggle-macOS-arm64.zip" ;;
      x86_64) ASSET="Gaggle-macOS-x64.zip" ;;
      *) echo "Unsupported macOS arch: $ARCH" >&2; exit 1 ;;
    esac
    NEED_UNZIP=1
    ;;
  Linux)
    case "$ARCH" in
      x86_64) ASSET="Gaggle-linux-x86_64.tar.gz" ;;
      aarch64|arm64) ASSET="Gaggle-linux-aarch64.tar.gz" ;;
      *) echo "Unsupported Linux arch: $ARCH" >&2; exit 1 ;;
    esac
    NEED_UNZIP=0
    ;;
  *)
    echo "Unsupported OS: $OS" >&2
    exit 1
    ;;
esac

if [[ -n "${GAGGLE_VERSION:-}" ]]; then
  BASE_URL="https://github.com/${REPO}/releases/download/${GAGGLE_VERSION}"
else
  BASE_URL="https://github.com/${REPO}/releases/latest/download"
fi

TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

echo "Downloading ${ASSET}..."
curl -fL --progress-bar -o "$TMP_DIR/$ASSET" "${BASE_URL}/${ASSET}"

if [[ "$NEED_UNZIP" -eq 1 ]]; then
  if ! command -v unzip >/dev/null 2>&1; then
    echo "Error: 'unzip' is required on macOS." >&2
    exit 1
  fi
  unzip -q "$TMP_DIR/$ASSET" -d "$TMP_DIR/extracted"
else
  mkdir -p "$TMP_DIR/extracted"
  tar -xzf "$TMP_DIR/$ASSET" -C "$TMP_DIR/extracted"
fi

# Find binaries anywhere in extracted tree
GAGGLE_BIN="$(find "$TMP_DIR/extracted" -type f -name 'gaggle' -perm -111 | head -n1)"
DESKTOP_BIN="$(find "$TMP_DIR/extracted" -type f -name 'gaggle-desktop' -perm -111 | head -n1)"

if [[ -z "$GAGGLE_BIN" ]]; then
  echo "Error: gaggle binary not found in archive." >&2
  exit 1
fi

install -m 755 "$GAGGLE_BIN" "$BIN_DIR/gaggle"
if [[ -n "$DESKTOP_BIN" ]]; then
  install -m 755 "$DESKTOP_BIN" "$BIN_DIR/gaggle-desktop"
fi

echo "Installed to $BIN_DIR"
echo "Ensure $BIN_DIR is on your PATH, then run: gaggle-desktop"
