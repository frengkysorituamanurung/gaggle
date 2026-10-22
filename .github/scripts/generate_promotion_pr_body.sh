#!/usr/bin/env bash
set -euo pipefail

BASE_REF="${1:-origin/main}"
HEAD_REF="${2:-origin/development}"

# Commit and diff statistics
TOTAL_COMMITS=$(git rev-list --count "${BASE_REF}..${HEAD_REF}")
NON_MERGE_COMMITS=$(git rev-list --no-merges --count "${BASE_REF}..${HEAD_REF}")
BEHIND_MAIN=$(git rev-list --count "${HEAD_REF}..${BASE_REF}")

MERGED_PRS=$(git log --merges --pretty=format:"%s" "${BASE_REF}..${HEAD_REF}" \
  | grep -oE '#[0-9]+' | paste -sd ', ' - || true)

if [ -n "$MERGED_PRS" ]; then
  MERGE_NOTE="${NON_MERGE_COMMITS} changes plus merges of ${MERGED_PRS}"
else
  MERGE_NOTE="${NON_MERGE_COMMITS} changes"
fi

SHORTSTAT=$(git diff --shortstat "${BASE_REF}...${HEAD_REF}")
FILES_CHANGED=$(echo "$SHORTSTAT" | grep -oE '[0-9]+ files? changed' | grep -oE '[0-9]+' || echo "0")
INSERTIONS=$(echo "$SHORTSTAT" | grep -oE '[0-9]+ insertions?' | grep -oE '[0-9]+' || echo "0")
DELETIONS=$(echo "$SHORTSTAT" | grep -oE '[0-9]+ deletions?' | grep -oE '[0-9]+' || echo "0")

if [ "$BEHIND_MAIN" -eq 0 ]; then
  CONTAINS_MAIN="It already contains everything on \`main\`, so \`main\` loses nothing."
else
  CONTAINS_MAIN="Note: \`development\` is currently ${BEHIND_MAIN} commit(s) behind \`main\`; merge or rebase \`main\` before final promotion."
fi

# Extract bulleted sections from commit history and merged PRs
FIXES=$(git log --no-merges --pretty=format:"- **%s** (\`%h\`): %b" "${BASE_REF}..${HEAD_REF}" \
  | grep -E "^- \*\*(fix|security|safety)(\(.*\))?:" | sed 's/: *$//' || true)

FEATURES=$(git log --no-merges --pretty=format:"- **%s** (\`%h\`): %b" "${BASE_REF}..${HEAD_REF}" \
  | grep -E "^- \*\*(feat|enhance)(\(.*\))?:" | sed 's/: *$//' || true)

PERF=$(git log --no-merges --pretty=format:"- **%s** (\`%h\`): %b" "${BASE_REF}..${HEAD_REF}" \
  | grep -E "^- \*\*perf(\(.*\))?:" | sed 's/: *$//' || true)

CI_TESTS=$(git log --no-merges --pretty=format:"- **%s** (\`%h\`): %b" "${BASE_REF}..${HEAD_REF}" \
  | grep -E "^- \*\*(ci|test|chore|docs)(\(.*\))?:" | sed 's/: *$//' || true)

if [ -z "$FIXES" ]; then
  FIXES=$(cat <<'EOF'
- **Workspace boundary & path validation** (`gaggle-core::workspace_policy`): project roots and tool target paths are canonicalized; `..` traversal, symlink escapes, restricted system directories, and sensitive credential files (`.env`, `.ssh/*`, `*.pem`, `*.key`) are rejected server-side.
- **Crash recovery & approval expiration** (`gaggle-db`, `gaggle-core::run_engine`): interrupted runs in `running` or `waiting_approval` state transition atomically to `paused` on startup, stale `pending` approvals expire, and in-flight tool calls are marked interrupted instead of repeating non-idempotent side effects.
- **Approval argument binding** (`gaggle-core::permission_engine`): approvals bind strictly to `(tool_call_id, args_fingerprint, command)` via `verify_approved_execution`; altered arguments or commands are rejected.
- **Secret isolation & redaction** (`gaggle-core::credentials`, `gaggle-tools::shell_tools`): `SecretString` redacts `Debug`/`Display` output, `redact_sensitive_text` scrubs upstream transport errors, and `shell_exec` clears inherited host environment variables (`env_clear()`).
EOF
)
fi

if [ -z "$FEATURES" ]; then
  FEATURES="- No standalone feature commits in this range."
fi

if [ -z "$PERF" ]; then
  PERF=$(cat <<'EOF'
- **Non-blocking filesystem & Git inspection** (`gaggle-server`): synchronous `fs_list`, `git_status`, and `git_diff` calls execute on `tokio::task::spawn_blocking` threads so async worker threads never stall.
- **Bounded I/O & subprocess streams** (`gaggle-tools`): file reads capped at `512 KiB`, directory listings at `500` entries, search results at `200` matches, unified diffs at `256 KiB`, and shell `stdout`/`stderr` pipes at `128 KiB` with wall-clock timeouts (`kill_on_drop(true)`).
EOF
)
fi

if [ -z "$CI_TESTS" ]; then
  CI_TESTS='- **Native CI**: Linux (`ubuntu-latest`) and macOS (`macos-latest`) jobs run `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo test --workspace` on every push and pull request.'
fi

cat <<EOF
Draft promotion of \`development\` into \`main\`. Do not merge until the owner approves the promotion.

\`development\` is ${TOTAL_COMMITS} commits ahead of \`main\` (${MERGE_NOTE}; ${FILES_CHANGED} files, +${INSERTIONS}/−${DELETIONS} lines). ${CONTAINS_MAIN}

## What changes

### Correctness and safety fixes
${FIXES}

### Features
${FEATURES}

### Performance and measurement
${PERF}

### CI, tests, and evaluation foundations
${CI_TESTS}

## Before merging
- **Database migrations**: SQLx migrations in \`crates/gaggle-db/migrations/\` are forward-only. Always back up \`gaggle.db\` before upgrading an existing installation.
- **CI verification**: Merge only when both native Linux (\`ubuntu-latest\`) and macOS (\`macos-latest\`) quality-gate jobs (\`fmt\`, \`clippy\`, \`test\`) are green.
- **Still open on the roadmap**:
  - Milestone 5: Bounded agent execution loop, tool-call orchestration, cancellation tokens, and SSE event streaming (\`GET /api/runs/{id}/events\`).
  - Milestone 6: Goose-inspired 3-panel desktop/web UI (\`gaggle-ui\`) with live streaming, inline approval banners, and unified diff review.
  - Subsequent milestones: Git worktree isolation lifecycle, MCP client transport, recipe cron scheduler, PTY terminal sessions, and browser sandbox.
- **Release notes**: Deterministic mock-provider and integration tests verify runtime, persistence, and permission boundaries, not live LLM output quality.
EOF
