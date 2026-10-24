#!/usr/bin/env bash
# shellcheck disable=SC2016
set -euo pipefail

# Generate release notes from the commits between the previous tag and the
# release tag. The output is intentionally conservative: it does not invent
# compatibility or installation claims.

if [[ $# -ne 4 ]]; then
  echo "Usage: generate-release-notes.sh <tag> <version> <commit> <output-file>" >&2
  exit 2
fi

TAG="$1"
VERSION="$2"
COMMIT="$3"
OUTPUT="$4"
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

PREVIOUS_TAG="$(
  git -C "$ROOT_DIR" describe --tags --abbrev=0 "$TAG^" 2>/dev/null || true
)"
if [[ -n "$PREVIOUS_TAG" ]]; then
  RANGE="$PREVIOUS_TAG..$TAG"
else
  RANGE="$TAG"
fi

{
  printf '# Gaggle %s\n\n' "$VERSION"
  printf 'Source commit: `%s`\n\n' "$COMMIT"
  printf '## Changes\n\n'
  git -C "$ROOT_DIR" log --pretty=format:'- %s (`%h`)' "$RANGE"
  printf '\n\n## Artifacts and verification\n\n'
  printf '%s\n' \
    '- macOS arm64 and x64 ZIP bundles containing `Gaggle.app` are published when the native builds succeed.' \
    '- Linux x86_64 and aarch64 portable `.tar.gz` bundles are published when the native builds succeed.' \
    '- Each package has a matching `.sha256` checksum and a JSON metadata file.'
  printf '\n'
  if [[ -z "${PREVIOUS_TAG:-}" ]]; then
    printf 'This is the first tagged release in the repository.\n'
  else
    printf 'Changes are relative to `%s`.\n' "$PREVIOUS_TAG"
  fi
} > "$OUTPUT"

echo "$OUTPUT"
