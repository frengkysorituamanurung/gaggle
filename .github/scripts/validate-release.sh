#!/usr/bin/env bash
set -euo pipefail

# Verify that a release tag matches Cargo.toml and points at a commit that is
# already on main. Print the workspace version on success.

if [[ $# -lt 1 || $# -gt 2 ]]; then
  echo "Usage: validate-release.sh <tag> [commit-sha]" >&2
  exit 2
fi

TAG="$1"
COMMIT="${2:-$(git rev-parse HEAD)}"
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

VERSION="$(
  sed -n 's/^version = "\(.*\)"/\1/p' "$ROOT_DIR/Cargo.toml" | head -n 1
)"
if [[ -z "$VERSION" ]]; then
  echo "Could not read workspace version from Cargo.toml" >&2
  exit 1
fi

if [[ "$TAG" != "v$VERSION" ]]; then
  echo "Tag $TAG does not match workspace version v$VERSION" >&2
  exit 1
fi

HEAD_SHA="$(git -C "$ROOT_DIR" rev-parse HEAD)"
if [[ "$HEAD_SHA" != "$COMMIT" ]]; then
  echo "Checked-out commit $HEAD_SHA does not match release commit $COMMIT" >&2
  exit 1
fi

TAG_SHA="$(git -C "$ROOT_DIR" rev-list -n 1 "$TAG")"
if [[ "$TAG_SHA" != "$COMMIT" ]]; then
  echo "Tag $TAG points at $TAG_SHA, expected $COMMIT" >&2
  exit 1
fi

git -C "$ROOT_DIR" fetch --no-tags origin main:refs/remotes/origin/main
if ! git -C "$ROOT_DIR" merge-base --is-ancestor "$COMMIT" refs/remotes/origin/main; then
  echo "Release commit $COMMIT is not on main" >&2
  exit 1
fi

if [[ -n "$(git -C "$ROOT_DIR" status --porcelain)" ]]; then
  echo "Working tree is not clean" >&2
  exit 1
fi

echo "$VERSION"
