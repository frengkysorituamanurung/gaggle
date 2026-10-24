#!/usr/bin/env bash
set -euo pipefail

# Build a repeatable macOS ZIP distribution for Gaggle.
#
# This script is a thin, local-first wrapper around the repository's existing
# packaging scripts. It builds the release binaries, assembles and signs
# Gaggle.app, recreates the ZIP with ditto, writes a SHA-256 checksum, and
# validates the extracted bundle by starting the packaged server and calling
# /api/health.
#
# User data is never placed inside Gaggle.app. The default SQLite database lives
# at ~/Library/Application Support/gaggle/gaggle.db.

usage() {
  cat <<'USAGE'
Usage: scripts/package_macos_zip.sh [options]

Options:
  --target <triple>        aarch64-apple-darwin or x86_64-apple-darwin
                           (default: host architecture)
  --arch <arm64|x64>       Convenience alias for --target
  --version <version>      Release version (default: workspace Cargo.toml)
  --binary-dir <dir>       Use prebuilt release binaries instead of building
  --output-dir <dir>       Destination directory for Gaggle.app and the ZIP
                           (default: <repo>/dist)
  --commit <sha>           Commit recorded in Info.plist (default: git HEAD)
  --sign-identity <id>     Developer ID identity for codesign
                           (default: ad-hoc "-"; env GAGGLE_CODESIGN_IDENTITY)
  --no-sign                Skip codesign entirely
  --skip-build             Do not run cargo build (requires existing binaries)
  --skip-smoke-test        Skip the extracted /api/health launch check
  -h, --help               Show this help

Outputs:
  <output-dir>/Gaggle.app
  <output-dir>/Gaggle-macOS-<arm64|x64>.zip
  <output-dir>/Gaggle-macOS-<arm64|x64>.zip.sha256
  <output-dir>/Gaggle-macOS-<arm64|x64>.json

The script intentionally produces only a ZIP. It does not create .dmg or .pkg
installers, and it never disables Gatekeeper.
USAGE
}

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

TARGET=""
VERSION=""
BINARY_DIR=""
OUTPUT_DIR="$ROOT_DIR/dist"
COMMIT=""
SIGN_IDENTITY="${GAGGLE_CODESIGN_IDENTITY:--}"
SKIP_BUILD="false"
SKIP_SMOKE_TEST="false"
NO_SIGN="false"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --target)
      TARGET="${2:?--target requires a value}"
      shift 2
      ;;
    --arch)
      case "${2:?--arch requires a value}" in
        arm64|aarch64)
          TARGET="aarch64-apple-darwin"
          ;;
        x64|x86_64)
          TARGET="x86_64-apple-darwin"
          ;;
        *)
          echo "Unsupported --arch value: $2 (expected arm64 or x64)" >&2
          exit 2
          ;;
      esac
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
    --sign-identity)
      SIGN_IDENTITY="${2:?--sign-identity requires a value}"
      shift 2
      ;;
    --no-sign)
      NO_SIGN="true"
      shift
      ;;
    --skip-build)
      SKIP_BUILD="true"
      shift
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

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "package_macos_zip.sh must run on macOS" >&2
  exit 1
fi

for command in cargo codesign ditto plutil shasum; do
  if ! command -v "$command" >/dev/null 2>&1; then
    echo "Required command is missing: $command" >&2
    exit 1
  fi
done

if [[ -z "$TARGET" ]]; then
  case "$(uname -m)" in
    arm64)
      TARGET="aarch64-apple-darwin"
      ;;
    x86_64)
      TARGET="x86_64-apple-darwin"
      ;;
    *)
      echo "Unsupported host architecture: $(uname -m)" >&2
      exit 1
      ;;
  esac
fi

case "$TARGET" in
  aarch64-apple-darwin)
    ARCH_LABEL="arm64"
    ;;
  x86_64-apple-darwin)
    ARCH_LABEL="x64"
    ;;
  *)
    echo "Unsupported macOS target: $TARGET" >&2
    exit 2
    ;;
esac

if [[ -z "$VERSION" ]]; then
  VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' "$ROOT_DIR/Cargo.toml" | head -n 1)"
fi
if [[ -z "$VERSION" ]]; then
  echo "Could not determine the workspace version" >&2
  exit 1
fi

if [[ -z "$COMMIT" ]]; then
  COMMIT="$(git -C "$ROOT_DIR" rev-parse HEAD 2>/dev/null || echo unknown)"
fi

if [[ -z "$BINARY_DIR" ]]; then
  BINARY_DIR="$ROOT_DIR/target/$TARGET/release"
  if [[ "$SKIP_BUILD" != "true" ]]; then
    echo "==> Building release binaries for $TARGET"
    (
      cd "$ROOT_DIR"
      MACOSX_DEPLOYMENT_TARGET="${MACOSX_DEPLOYMENT_TARGET:-12.0}" \
        cargo build --release --locked \
          -p gaggle-cli \
          --bin gaggle \
          --bin gaggle-desktop \
          --target "$TARGET"
    )
  fi
else
  BINARY_DIR="$(cd "$BINARY_DIR" && pwd)"
fi

for binary in "$BINARY_DIR/gaggle" "$BINARY_DIR/gaggle-desktop"; do
  if [[ ! -x "$binary" ]]; then
    echo "Required binary is missing or not executable: $binary" >&2
    exit 1
  fi
done

mkdir -p "$OUTPUT_DIR"
OUTPUT_DIR="$(cd "$OUTPUT_DIR" && pwd)"

echo "==> Assembling Gaggle.app with .github/scripts/package-macos.sh"
ZIP_PATH="$(
  "$ROOT_DIR/.github/scripts/package-macos.sh" \
    --target "$TARGET" \
    --version "$VERSION" \
    --binary-dir "$BINARY_DIR" \
    --output-dir "$OUTPUT_DIR" \
    --commit "$COMMIT"
)"
APP_DIR="$OUTPUT_DIR/Gaggle.app"
INFO_PLIST="$APP_DIR/Contents/Info.plist"

if [[ ! -d "$APP_DIR" || ! -f "$INFO_PLIST" ]]; then
  echo "Gaggle.app or Info.plist is missing: $APP_DIR" >&2
  exit 1
fi

echo "==> Validating Info.plist"
plutil -lint "$INFO_PLIST" >/dev/null
BUNDLE_EXECUTABLE="$(plutil -extract CFBundleExecutable raw -o - "$INFO_PLIST")"
BUNDLE_IDENTIFIER="$(plutil -extract CFBundleIdentifier raw -o - "$INFO_PLIST")"
if [[ "$BUNDLE_EXECUTABLE" != "Gaggle" || -z "$BUNDLE_IDENTIFIER" ]]; then
  echo "Unexpected bundle metadata (CFBundleExecutable=$BUNDLE_EXECUTABLE, CFBundleIdentifier=$BUNDLE_IDENTIFIER)" >&2
  exit 1
fi

if [[ -n "$(find "$APP_DIR" -type f \( -name '*.db' -o -name '*.db-wal' -o -name '*.db-shm' -o -name '.env' \) -print -quit)" ]]; then
  echo "Gaggle.app contains user data or secrets" >&2
  exit 1
fi

echo "==> Verifying SQLite data stays outside the app bundle"
EXPECTED_DB_PATH="$HOME/Library/Application Support/gaggle/gaggle.db"
DEFAULT_DB_PATH="$("$APP_DIR/Contents/Resources/bin/gaggle" db-path)"
if [[ "$DEFAULT_DB_PATH" != "$EXPECTED_DB_PATH" ]]; then
  echo "Unexpected default database path: $DEFAULT_DB_PATH (expected $EXPECTED_DB_PATH)" >&2
  exit 1
fi

SIGNED="false"
if [[ "$NO_SIGN" == "true" ]]; then
  echo "==> Skipping codesign (--no-sign)"
else
  echo "==> Signing Gaggle.app with identity: $SIGN_IDENTITY"
  SIGN_ARGS=(--force --sign "$SIGN_IDENTITY")
  if [[ "$SIGN_IDENTITY" != "-" ]]; then
    SIGN_ARGS+=(--options runtime --timestamp)
  fi
  for binary in \
    "$APP_DIR/Contents/Resources/bin/gaggle" \
    "$APP_DIR/Contents/MacOS/Gaggle"; do
    codesign "${SIGN_ARGS[@]}" "$binary"
  done
  codesign "${SIGN_ARGS[@]}" "$APP_DIR"

  if codesign --verify --deep --strict --verbose=2 "$APP_DIR" >/dev/null 2>&1; then
    echo "==> codesign verification passed"
    SIGNED="true"
  else
    echo "codesign verification failed for $APP_DIR" >&2
    codesign --verify --deep --strict --verbose=2 "$APP_DIR" >&2 || true
    exit 1
  fi
fi

echo "==> Recreating ZIP with ditto"
rm -f "$ZIP_PATH"
ditto --norsrc --noextattr --noqtn -c -k --keepParent "$APP_DIR" "$ZIP_PATH"

echo "==> Validating extracted bundle and starting /api/health smoke test"
VALIDATE_ARGS=(--archive "$ZIP_PATH" --target "$TARGET")
if [[ "$SKIP_SMOKE_TEST" == "true" ]]; then
  VALIDATE_ARGS+=(--skip-smoke-test)
fi
"$ROOT_DIR/.github/scripts/validate-macos-bundle.sh" "${VALIDATE_ARGS[@]}"

GIT_REF="$(git -C "$ROOT_DIR" rev-parse --abbrev-ref HEAD 2>/dev/null || echo unknown)"
SOURCE_REF="$(git -C "$ROOT_DIR" describe --always --dirty 2>/dev/null || echo unknown)"

echo "==> Writing SHA-256 checksum and package metadata"
"$ROOT_DIR/.github/scripts/write-artifact-metadata.sh" \
  --package "$ZIP_PATH" \
  --artifact-name "Gaggle-macOS-$ARCH_LABEL" \
  --version "$VERSION" \
  --commit "$COMMIT" \
  --git-ref "$GIT_REF" \
  --source-ref "$SOURCE_REF" \
  --os "macOS" \
  --arch "$ARCH_LABEL" \
  --target "$TARGET" \
  --format "zip" \
  --signed "$SIGNED" \
  --notarized "false" >/dev/null

HASH="$(awk '{print $1}' "$ZIP_PATH.sha256")"

echo
echo "Package:    $ZIP_PATH"
echo "SHA-256:    $HASH"
echo "Data path:  $EXPECTED_DB_PATH"
echo "Note:       Local ad-hoc or Developer ID builds are not notarized."
echo "            Gatekeeper may require the standard Open confirmation flow."
