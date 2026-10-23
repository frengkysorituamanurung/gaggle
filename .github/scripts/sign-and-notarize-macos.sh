#!/usr/bin/env bash
set -euo pipefail

# Sign and notarize a macOS app bundle. This script runs only on macOS and only
# when the release workflow has confirmed that all signing secrets are present.

if [[ $# -ne 2 ]]; then
  echo "Usage: sign-and-notarize-macos.sh <Gaggle.app> <zip-path>" >&2
  exit 2
fi

APP_DIR="$1"
ZIP_PATH="$2"

for variable in APPLE_CERTIFICATE_BASE64 APPLE_CERTIFICATE_PASSWORD APPLE_ID APPLE_ID_PASSWORD APPLE_TEAM_ID; do
  if [[ -z "${!variable:-}" ]]; then
    echo "Required signing variable is missing: $variable" >&2
    exit 1
  fi
done

if [[ ! -d "$APP_DIR" ]]; then
  echo "App bundle does not exist: $APP_DIR" >&2
  exit 1
fi

WORK_DIR="$(mktemp -d "${RUNNER_TEMP:-/tmp}/gaggle-signing.XXXXXX")"
KEYCHAIN_PATH="$WORK_DIR/gaggle-signing.keychain-db"
KEYCHAIN_PASSWORD="$(openssl rand -hex 32)"
read -r -a ORIGINAL_KEYCHAINS <<< "$(security list-keychains -d user | tr -d '"')"

cleanup() {
  security delete-keychain "$KEYCHAIN_PATH" >/dev/null 2>&1 || true
  rm -rf "$WORK_DIR"
}
trap cleanup EXIT

printf '%s' "$APPLE_CERTIFICATE_BASE64" | base64 -D > "$WORK_DIR/certificate.p12"

security create-keychain -p "$KEYCHAIN_PASSWORD" "$KEYCHAIN_PATH"
security set-keychain-settings -lut 21600 "$KEYCHAIN_PATH"
security unlock-keychain -p "$KEYCHAIN_PASSWORD" "$KEYCHAIN_PATH"
security import "$WORK_DIR/certificate.p12" \
  -k "$KEYCHAIN_PATH" \
  -P "$APPLE_CERTIFICATE_PASSWORD" \
  -T /usr/bin/codesign >/dev/null
security set-key-partition-list \
  -S apple-tool:,apple: \
  -s \
  -k "$KEYCHAIN_PASSWORD" \
  "$KEYCHAIN_PATH" >/dev/null
security list-keychains -d user -s "$KEYCHAIN_PATH" "${ORIGINAL_KEYCHAINS[@]}"

IDENTITY="$(
  security find-identity -v -p codesigning "$KEYCHAIN_PATH" \
    | sed -n 's/.*"\(.*\)"/\1/p' \
    | head -n 1
)"
if [[ -z "$IDENTITY" ]]; then
  echo "No code-signing identity found in the temporary keychain" >&2
  exit 1
fi

for binary in "$APP_DIR/Contents/Resources/bin/gaggle" "$APP_DIR/Contents/MacOS/Gaggle"; do
  codesign \
    --force \
    --timestamp \
    --options runtime \
    --sign "$IDENTITY" \
    "$binary"
done
codesign \
  --force \
  --timestamp \
  --options runtime \
  --sign "$IDENTITY" \
  "$APP_DIR"
codesign --verify --deep --strict --verbose=2 "$APP_DIR"

rm -f "$ZIP_PATH"
ditto -c -k --sequesterRsrc --keepParent "$APP_DIR" "$ZIP_PATH"

xcrun notarytool submit "$ZIP_PATH" \
  --apple-id "$APPLE_ID" \
  --password "$APPLE_ID_PASSWORD" \
  --team-id "$APPLE_TEAM_ID" \
  --wait

xcrun stapler staple "$APP_DIR"
xcrun stapler validate "$APP_DIR"

# Recreate the zip after stapling so the distributed archive contains the
# notarization ticket.
rm -f "$ZIP_PATH"
ditto -c -k --sequesterRsrc --keepParent "$APP_DIR" "$ZIP_PATH"

echo "Signed and notarized $APP_DIR"
