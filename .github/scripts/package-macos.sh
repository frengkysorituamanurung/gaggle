#!/usr/bin/env bash
set -euo pipefail

# Assemble a self-contained macOS application bundle.
#
# Gaggle does not use a native desktop webview. The bundle therefore contains
# the `gaggle-desktop` launcher (renamed to `Gaggle`), the `gaggle` CLI, and the
# static UI assets embedded in the Rust server. The launcher starts the local
# server and opens the user's browser.

usage() {
  cat <<'USAGE'
Usage: package-macos.sh --target <target> --version <version> --binary-dir <dir> --output-dir <dir> [options]

Options:
  --target <triple>       aarch64-apple-darwin or x86_64-apple-darwin
  --version <version>     Release version, for example 0.1.0
  --binary-dir <dir>      Directory containing release `gaggle` and `gaggle-desktop`
  --output-dir <dir>      Destination directory for the zip
  --commit <sha>          Git commit recorded in Info.plist (default: unknown)
  -h, --help              Show this help
USAGE
}

TARGET=""
VERSION=""
BINARY_DIR=""
OUTPUT_DIR=""
COMMIT="unknown"

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

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
APP_NAME="Gaggle"
APP_DIR="$OUTPUT_DIR/$APP_NAME.app"
ZIP_PATH="$OUTPUT_DIR/$APP_NAME-macOS-$ARCH_LABEL.zip"

for binary in "$BINARY_DIR/gaggle" "$BINARY_DIR/gaggle-desktop"; do
  if [[ ! -x "$binary" ]]; then
    echo "Required binary is missing or not executable: $binary" >&2
    exit 1
  fi
done

if [[ ! -f "$ROOT_DIR/assets/Gaggle.icns" ]]; then
  echo "Application icon is missing: $ROOT_DIR/assets/Gaggle.icns" >&2
  exit 1
fi

rm -rf "$APP_DIR" "$ZIP_PATH"
mkdir -p "$APP_DIR/Contents/MacOS" "$APP_DIR/Contents/Resources/bin"

# macOS filesystems are case-insensitive by default. Keep the CLI in
# Resources/bin so it cannot overwrite the `Gaggle` desktop launcher.
install -m 755 "$BINARY_DIR/gaggle-desktop" "$APP_DIR/Contents/MacOS/Gaggle"
install -m 755 "$BINARY_DIR/gaggle" "$APP_DIR/Contents/Resources/bin/gaggle"
install -m 644 "$ROOT_DIR/assets/Gaggle.icns" "$APP_DIR/Contents/Resources/AppIcon.icns"
install -m 644 "$ROOT_DIR/docs/assets/gaggle-logo.svg" "$APP_DIR/Contents/Resources/gaggle-logo.svg"
install -m 644 "$ROOT_DIR/LICENSE" "$APP_DIR/Contents/Resources/LICENSE"

cat > "$APP_DIR/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleDevelopmentRegion</key>
  <string>en</string>
  <key>CFBundleDisplayName</key>
  <string>$APP_NAME</string>
  <key>CFBundleExecutable</key>
  <string>Gaggle</string>
  <key>CFBundleIconFile</key>
  <string>AppIcon</string>
  <key>CFBundleIdentifier</key>
  <string>io.github.frengkysorituamanurung.gaggle</string>
  <key>CFBundleInfoDictionaryVersion</key>
  <string>6.0</string>
  <key>CFBundleName</key>
  <string>$APP_NAME</string>
  <key>CFBundlePackageType</key>
  <string>APPL</string>
  <key>CFBundleShortVersionString</key>
  <string>$VERSION</string>
  <key>CFBundleVersion</key>
  <string>$VERSION</string>
  <key>LSApplicationCategoryType</key>
  <string>public.app-category.developer-tools</string>
  <key>LSMinimumSystemVersion</key>
  <string>12.0</string>
  <key>NSHighResolutionCapable</key>
  <true/>
  <key>NSHumanReadableCopyright</key>
  <string>Copyright Gaggle contributors. Licensed under Apache-2.0.</string>
  <key>GaggleGitCommit</key>
  <string>$COMMIT</string>
</dict>
</plist>
PLIST

plutil -lint "$APP_DIR/Contents/Info.plist" >/dev/null

# Remove extended attributes so the distribution zip contains only the bundle.
xattr -cr "$APP_DIR"

# Ad-hoc sign the bundle. This keeps unsigned CI/PR artifacts from being
# reported as "damaged" by Gatekeeper while still allowing the optional
# Developer ID signing step to replace this signature later.
codesign --force --sign - "$APP_DIR/Contents/Resources/bin/gaggle"
codesign --force --sign - "$APP_DIR/Contents/MacOS/Gaggle"
codesign --force --sign - "$APP_DIR"

# `ditto` preserves the bundle structure and permissions expected by macOS.
ditto --norsrc --noextattr --noqtn -c -k --keepParent "$APP_DIR" "$ZIP_PATH"

echo "$ZIP_PATH"
