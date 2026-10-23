#!/usr/bin/env bash
set -euo pipefail

# Validate all package checksums and generate a combined manifest plus a
# SHA256SUMS file for the workflow artifacts.

usage() {
  cat <<'USAGE'
Usage: generate-build-manifest.sh --dist <dir> --version <version> \
  --commit <sha> --git-ref <ref> --workflow-run-url <url> \
  [--expected-count <n>]
USAGE
}

DIST_DIR=""
VERSION=""
COMMIT=""
GIT_REF=""
WORKFLOW_RUN_URL=""
EXPECTED_COUNT=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --dist)
      DIST_DIR="${2:?--dist requires a value}"
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
    --workflow-run-url)
      WORKFLOW_RUN_URL="${2:?--workflow-run-url requires a value}"
      shift 2
      ;;
    --expected-count)
      EXPECTED_COUNT="${2:?--expected-count requires a value}"
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

if [[ -z "$DIST_DIR" || -z "$VERSION" || -z "$COMMIT" || -z "$GIT_REF" || -z "$WORKFLOW_RUN_URL" ]]; then
  usage >&2
  exit 2
fi

if [[ -n "$EXPECTED_COUNT" && ! "$EXPECTED_COUNT" =~ ^[1-9][0-9]*$ ]]; then
  echo "Invalid expected artifact count: $EXPECTED_COUNT" >&2
  exit 2
fi

if [[ ! -d "$DIST_DIR" ]]; then
  echo "Distribution directory does not exist: $DIST_DIR" >&2
  exit 1
fi

shopt -s nullglob
PACKAGES=(
  "$DIST_DIR"/*.zip
  "$DIST_DIR"/*.tar.gz
)
shopt -u nullglob

if [[ ${#PACKAGES[@]} -eq 0 ]]; then
  echo "No package archives found in $DIST_DIR" >&2
  exit 1
fi

if [[ -n "$EXPECTED_COUNT" && ${#PACKAGES[@]} -ne "$EXPECTED_COUNT" ]]; then
  echo "Expected $EXPECTED_COUNT package archives, found ${#PACKAGES[@]} in $DIST_DIR" >&2
  printf '  %s\n' "${PACKAGES[@]}" >&2
  exit 1
fi

: > "$DIST_DIR/SHA256SUMS"
for package in "${PACKAGES[@]}"; do
  checksum_file="$package.sha256"
  if [[ ! -f "$checksum_file" ]]; then
    echo "Missing checksum file: $checksum_file" >&2
    exit 1
  fi
  (
    cd "$DIST_DIR"
    if command -v sha256sum >/dev/null 2>&1; then
      sha256sum -c "$(basename "$checksum_file")"
    else
      shasum -a 256 -c "$(basename "$checksum_file")"
    fi
  )
  cat "$checksum_file" >> "$DIST_DIR/SHA256SUMS"
done

DIST_DIR="$DIST_DIR" \
VERSION="$VERSION" \
COMMIT="$COMMIT" \
GIT_REF="$GIT_REF" \
WORKFLOW_RUN_URL="$WORKFLOW_RUN_URL" \
python3 - <<'PY'
import glob
import json
import os

dist_dir = os.environ["DIST_DIR"]
metadata = []
for path in sorted(glob.glob(os.path.join(dist_dir, "*.json"))):
    if os.path.basename(path) == "build-manifest.json":
        continue
    with open(path, encoding="utf-8") as handle:
        metadata.append(json.load(handle))

manifest = {
    "version": os.environ["VERSION"],
    "git_commit": os.environ["COMMIT"],
    "git_ref": os.environ["GIT_REF"],
    "workflow_run_url": os.environ["WORKFLOW_RUN_URL"],
    "artifacts": sorted(metadata, key=lambda item: item.get("artifact", "")),
}
with open(os.path.join(dist_dir, "build-manifest.json"), "w", encoding="utf-8") as handle:
    json.dump(manifest, handle, indent=2, sort_keys=True)
    handle.write("\n")
PY

echo "$DIST_DIR/build-manifest.json"
echo "$DIST_DIR/SHA256SUMS"
