#!/usr/bin/env bash
set -euo pipefail

# Generate a SHA-256 file and a machine-readable metadata file for one package.

usage() {
  cat <<'USAGE'
Usage: write-artifact-metadata.sh --package <file> --artifact-name <name> \
  --version <version> --commit <sha> --git-ref <ref> --source-ref <ref> \
  --os <name> --arch <arch> --target <triple> --format <format> \
  --signed <true|false> --notarized <true|false> [--workflow-run-url <url>]

Writes:
  <package>.sha256
  <artifact-name>.json (next to the package)
USAGE
}

PACKAGE=""
ARTIFACT_NAME=""
VERSION=""
COMMIT=""
GIT_REF=""
SOURCE_REF=""
OS_NAME=""
ARCH=""
TARGET=""
FORMAT=""
SIGNED="false"
NOTARIZED="false"
WORKFLOW_RUN_URL=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --package)
      PACKAGE="${2:?--package requires a value}"
      shift 2
      ;;
    --artifact-name)
      ARTIFACT_NAME="${2:?--artifact-name requires a value}"
      shift 2
      ;;
    --version)
      VERSION="${2:?--version requires a value}"
      shift 2
      ;;
    --commit)
      COMMIT="${2:?--commit requires a value}"
      shift 2
      ;;
    --git-ref)
      GIT_REF="${2:?--git-ref requires a value}"
      shift 2
      ;;
    --source-ref)
      SOURCE_REF="${2:?--source-ref requires a value}"
      shift 2
      ;;
    --os)
      OS_NAME="${2:?--os requires a value}"
      shift 2
      ;;
    --arch)
      ARCH="${2:?--arch requires a value}"
      shift 2
      ;;
    --target)
      TARGET="${2:?--target requires a value}"
      shift 2
      ;;
    --format)
      FORMAT="${2:?--format requires a value}"
      shift 2
      ;;
    --signed)
      SIGNED="${2:?--signed requires a value}"
      shift 2
      ;;
    --notarized)
      NOTARIZED="${2:?--notarized requires a value}"
      shift 2
      ;;
    --workflow-run-url)
      WORKFLOW_RUN_URL="${2:?--workflow-run-url requires a value}"
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

for required in PACKAGE ARTIFACT_NAME VERSION COMMIT GIT_REF SOURCE_REF OS_NAME ARCH TARGET FORMAT; do
  if [[ -z "${!required}" ]]; then
    echo "Missing required value: $required" >&2
    usage >&2
    exit 2
  fi
done

if [[ ! -f "$PACKAGE" ]]; then
  echo "Package does not exist: $PACKAGE" >&2
  exit 1
fi

OUTPUT_DIR="$(cd "$(dirname "$PACKAGE")" && pwd)"
PACKAGE_BASENAME="$(basename "$PACKAGE")"
CHECKSUM_FILE="$PACKAGE.sha256"
METADATA_FILE="$OUTPUT_DIR/$ARTIFACT_NAME.json"

if command -v sha256sum >/dev/null 2>&1; then
  HASH="$(sha256sum "$PACKAGE" | awk '{print $1}')"
else
  HASH="$(shasum -a 256 "$PACKAGE" | awk '{print $1}')"
fi
printf '%s  %s\n' "$HASH" "$PACKAGE_BASENAME" > "$CHECKSUM_FILE"

PACKAGE_BASENAME="$PACKAGE_BASENAME" \
HASH="$HASH" \
METADATA_FILE="$METADATA_FILE" \
ARTIFACT_NAME="$ARTIFACT_NAME" \
VERSION="$VERSION" \
COMMIT="$COMMIT" \
GIT_REF="$GIT_REF" \
SOURCE_REF="$SOURCE_REF" \
OS_NAME="$OS_NAME" \
ARCH="$ARCH" \
TARGET="$TARGET" \
FORMAT="$FORMAT" \
SIGNED="$SIGNED" \
NOTARIZED="$NOTARIZED" \
WORKFLOW_RUN_URL="$WORKFLOW_RUN_URL" \
python3 - <<'PY'
import json
import os
from datetime import datetime, timezone

signed = os.environ["SIGNED"] == "true"
notarized = os.environ["NOTARIZED"] == "true"
metadata = {
    "artifact": os.environ["PACKAGE_BASENAME"],
    "artifact_name": os.environ["ARTIFACT_NAME"],
    "version": os.environ["VERSION"],
    "git_commit": os.environ["COMMIT"],
    "git_ref": os.environ["GIT_REF"],
    "source_ref": os.environ["SOURCE_REF"],
    "os": os.environ["OS_NAME"],
    "arch": os.environ["ARCH"],
    "target": os.environ["TARGET"],
    "format": os.environ["FORMAT"],
    "sha256": os.environ["HASH"],
    "signed": signed,
    "notarized": notarized,
    "workflow_run_url": os.environ["WORKFLOW_RUN_URL"],
    "generated_at": datetime.now(timezone.utc).replace(microsecond=0).isoformat().replace("+00:00", "Z"),
}
with open(os.environ["METADATA_FILE"], "w", encoding="utf-8") as handle:
    json.dump(metadata, handle, indent=2, sort_keys=True)
    handle.write("\n")
PY

echo "$CHECKSUM_FILE"
echo "$METADATA_FILE"
