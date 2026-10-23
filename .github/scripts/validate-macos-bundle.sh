#!/usr/bin/env bash
set -euo pipefail

# Validate the macOS zip produced by package-macos.sh before it is uploaded.
#
# This intentionally checks the extracted archive, not the staging directory,
# so bundle-path bugs and missing resources are caught.

usage() {
  cat <<'USAGE'
Usage: validate-macos-bundle.sh --archive <zip> --target <triple> [--skip-smoke-test]

Options:
  --archive <zip>         Zip containing Gaggle.app
  --target <triple>       aarch64-apple-darwin or x86_64-apple-darwin
  --skip-smoke-test       Skip starting the packaged server
  --require-signature     Fail when the bundle is unsigned
  -h, --help              Show this help
USAGE
}

ARCHIVE=""
TARGET=""
SKIP_SMOKE_TEST="false"
REQUIRE_SIGNATURE="false"

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
    --require-signature)
      REQUIRE_SIGNATURE="true"
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
  aarch64-apple-darwin)
    EXPECTED_ARCH="arm64"
    ;;
  x86_64-apple-darwin)
    EXPECTED_ARCH="x86_64"
    ;;
  *)
    echo "Unsupported macOS target: $TARGET" >&2
    exit 2
    ;;
esac

if [[ ! -f "$ARCHIVE" ]]; then
  echo "Archive does not exist: $ARCHIVE" >&2
  exit 1
fi

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
WORK_DIR="$(mktemp -d "${TMPDIR:-/tmp}/gaggle-macos-validate.XXXXXX")"
trap 'rm -rf "$WORK_DIR"' EXIT

if unzip -Z1 "$ARCHIVE" | grep -Eq '^/|(^|/)\.\.(/|$)'; then
  echo "Archive contains an absolute or parent-relative path" >&2
  exit 1
fi

ditto -x -k "$ARCHIVE" "$WORK_DIR"
APP_DIR="$WORK_DIR/Gaggle.app"
INFO_PLIST="$APP_DIR/Contents/Info.plist"
MAIN_BINARY="$APP_DIR/Contents/MacOS/Gaggle"
CLI_BINARY="$APP_DIR/Contents/Resources/bin/gaggle"

if [[ ! -d "$APP_DIR" ]]; then
  echo "Gaggle.app is missing from the archive" >&2
  exit 1
fi
if [[ ! -f "$INFO_PLIST" ]]; then
  echo "Info.plist is missing" >&2
  exit 1
fi

plutil -lint "$INFO_PLIST" >/dev/null
BUNDLE_EXECUTABLE="$(plutil -extract CFBundleExecutable raw -o - "$INFO_PLIST")"
BUNDLE_IDENTIFIER="$(plutil -extract CFBundleIdentifier raw -o - "$INFO_PLIST")"
BUNDLE_VERSION="$(plutil -extract CFBundleShortVersionString raw -o - "$INFO_PLIST")"
MINIMUM_SYSTEM_VERSION="$(plutil -extract LSMinimumSystemVersion raw -o - "$INFO_PLIST")"

if [[ "$BUNDLE_EXECUTABLE" != "Gaggle" ]]; then
  echo "Unexpected CFBundleExecutable: $BUNDLE_EXECUTABLE" >&2
  exit 1
fi
if [[ -z "$BUNDLE_IDENTIFIER" || -z "$BUNDLE_VERSION" ]]; then
  echo "Bundle identifier or version is empty" >&2
  exit 1
fi
if [[ "$MINIMUM_SYSTEM_VERSION" != "12.0" ]]; then
  echo "Unexpected LSMinimumSystemVersion: $MINIMUM_SYSTEM_VERSION" >&2
  exit 1
fi
if [[ ! -x "$MAIN_BINARY" || ! -x "$CLI_BINARY" ]]; then
  echo "Expected executables are missing or not executable" >&2
  exit 1
fi

for binary in "$MAIN_BINARY" "$CLI_BINARY"; do
  if ! file "$binary" | grep -q "$EXPECTED_ARCH"; then
    echo "Unexpected architecture for $binary:" >&2
    file "$binary" >&2
    exit 1
  fi

  MINIMUM_OS="$(
    otool -l "$binary" \
      | awk '/LC_BUILD_VERSION/{found=1} found && /minos/{print $2; exit}' \
      || true
  )"
  if [[ -z "$MINIMUM_OS" ]]; then
    MINIMUM_OS="$(
      otool -l "$binary" \
        | awk '/LC_VERSION_MIN_MACOSX/{found=1} found && /version/{print $2; exit}' \
        || true
    )"
  fi
  if [[ -n "$MINIMUM_OS" && "${MINIMUM_OS%%.*}" -gt 12 ]]; then
    echo "Binary requires macOS $MINIMUM_OS, expected macOS 12.0 or older: $binary" >&2
    exit 1
  fi

  unexpected_dependencies="$(
    otool -L "$binary" \
      | tail -n +2 \
      | awk '{print $1}' \
      | grep -Ev '^(/usr/lib/|/System/Library/|@rpath/|@loader_path/|@executable_path/)' \
      || true
  )"
  if [[ -n "$unexpected_dependencies" ]]; then
    echo "Unexpected non-system dependencies in $binary:" >&2
    echo "$unexpected_dependencies" >&2
    exit 1
  fi
done

for resource in \
  "$APP_DIR/Contents/Resources/AppIcon.icns" \
  "$APP_DIR/Contents/Resources/gaggle-logo.svg" \
  "$APP_DIR/Contents/Resources/LICENSE" \
  "$APP_DIR/Contents/Resources/bin/gaggle"; do
  if [[ ! -f "$resource" ]]; then
    echo "Missing bundle resource: $resource" >&2
    exit 1
  fi
done

if find "$APP_DIR" -type d -name .git -o -type f -name '*.db' -o -type f -name '.env' -o -type f -name '._*' | grep -q .; then
  echo "Bundle contains development or user data files" >&2
  exit 1
fi

if codesign --verify --deep --strict --verbose=2 "$APP_DIR" >/dev/null 2>&1; then
  echo "macOS code signature verified"
  if [[ "$REQUIRE_SIGNATURE" == "true" ]]; then
    xcrun stapler validate "$APP_DIR"
  fi
elif [[ "$REQUIRE_SIGNATURE" == "true" ]]; then
  echo "macOS bundle is not validly signed" >&2
  codesign --verify --deep --strict --verbose=2 "$APP_DIR" >&2 || true
  exit 1
else
  echo "macOS bundle is unsigned (expected for local and unsigned CI builds)"
fi

"$MAIN_BINARY" --version >/dev/null
"$CLI_BINARY" --version >/dev/null

if [[ "$SKIP_SMOKE_TEST" != "true" ]]; then
  "$ROOT_DIR/.github/scripts/smoke-test-server.sh" "$MAIN_BINARY"
fi

echo "Validated $ARCHIVE ($EXPECTED_ARCH)"
