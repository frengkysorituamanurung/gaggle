#!/usr/bin/env bash
set -euo pipefail

# Validate the Linux archive produced by package-linux.sh.

usage() {
  cat <<'USAGE'
Usage: validate-linux-package.sh --archive <tar.gz> --target <triple> [--skip-smoke-test]

Options:
  --archive <tar.gz>      Archive containing Gaggle-linux-<arch>/
  --target <triple>       x86_64-unknown-linux-gnu or aarch64-unknown-linux-gnu
  --skip-smoke-test       Skip starting the packaged server
  -h, --help              Show this help
USAGE
}

ARCHIVE=""
TARGET=""
SKIP_SMOKE_TEST="false"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --archive)
      ARCHIVE="${2:?--archive requires a value}"
      shift 2
      ;;
    --target)
      TARGET="${2:?--target requires a value}"
      shift 2
      ;;
    --skip-smoke-test)
      SKIP_SMOKE_TEST="true"
      shift
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

if [[ -z "$ARCHIVE" || -z "$TARGET" ]]; then
  usage >&2
  exit 2
fi

case "$TARGET" in
  x86_64-unknown-linux-gnu)
    ARCH_LABEL="x86_64"
    EXPECTED_FILE_ARCH="x86-64"
    ;;
  aarch64-unknown-linux-gnu)
    ARCH_LABEL="aarch64"
    EXPECTED_FILE_ARCH="ARM aarch64"
    ;;
  *)
    echo "Unsupported Linux target: $TARGET" >&2
    exit 2
    ;;
esac

if [[ ! -f "$ARCHIVE" ]]; then
  echo "Archive does not exist: $ARCHIVE" >&2
  exit 1
fi

if tar -tzf "$ARCHIVE" | grep -Eq '^/|(^|/)\.\.(/|$)'; then
  echo "Archive contains an absolute or parent-relative path" >&2
  exit 1
fi

if [[ -f "$ARCHIVE.sha256" ]]; then
  (
    cd "$(dirname "$ARCHIVE")"
    sha256sum -c "$(basename "$ARCHIVE").sha256"
  )
fi

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
WORK_DIR="$(mktemp -d "${TMPDIR:-/tmp}/gaggle-linux-validate.XXXXXX")"
trap 'rm -rf "$WORK_DIR"' EXIT

tar -xzf "$ARCHIVE" -C "$WORK_DIR"
PACKAGE_ROOT="$WORK_DIR/Gaggle-linux-$ARCH_LABEL"

if [[ ! -d "$PACKAGE_ROOT" ]]; then
  echo "Expected package root is missing: Gaggle-linux-$ARCH_LABEL" >&2
  exit 1
fi

for relative_path in \
  bin/gaggle \
  bin/gaggle-desktop \
  install.sh \
  share/applications/gaggle.desktop \
  share/icons/hicolor/scalable/apps/gaggle.svg \
  share/gaggle/gaggle-logo.svg \
  README.txt \
  README.md \
  LICENSE; do
  if [[ ! -e "$PACKAGE_ROOT/$relative_path" ]]; then
    echo "Missing package file: $relative_path" >&2
    exit 1
  fi
done

if [[ ! -x "$PACKAGE_ROOT/bin/gaggle" || ! -x "$PACKAGE_ROOT/bin/gaggle-desktop" || ! -x "$PACKAGE_ROOT/install.sh" ]]; then
  echo "Expected executable permissions are missing" >&2
  exit 1
fi

if ! sh -n "$PACKAGE_ROOT/install.sh"; then
  echo "install.sh has invalid shell syntax" >&2
  exit 1
fi

if ! file "$PACKAGE_ROOT/bin/gaggle-desktop" | grep -q "$EXPECTED_FILE_ARCH"; then
  echo "Unexpected architecture for bin/gaggle-desktop:" >&2
  file "$PACKAGE_ROOT/bin/gaggle-desktop" >&2
  exit 1
fi

for binary in "$PACKAGE_ROOT/bin/gaggle" "$PACKAGE_ROOT/bin/gaggle-desktop"; do
  if ldd "$binary" 2>&1 | grep -q 'not found'; then
    echo "Missing shared library for $binary" >&2
    ldd "$binary" >&2
    exit 1
  fi
done

if ! grep -q '^Exec=gaggle-desktop$' "$PACKAGE_ROOT/share/applications/gaggle.desktop"; then
  echo "Desktop entry does not launch gaggle-desktop" >&2
  exit 1
fi

"$PACKAGE_ROOT/bin/gaggle" --version >/dev/null
"$PACKAGE_ROOT/bin/gaggle-desktop" --version >/dev/null

if [[ "$SKIP_SMOKE_TEST" != "true" ]]; then
  "$ROOT_DIR/.github/scripts/smoke-test-server.sh" "$PACKAGE_ROOT/bin/gaggle-desktop"
fi

echo "Validated $ARCHIVE ($ARCH_LABEL)"
