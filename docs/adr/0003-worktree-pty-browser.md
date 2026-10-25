# ADR 0003 — Worktree, PTY, and Browser

- Status: Accepted
- Date: 2026-10-25

## Context

Agent file changes, terminal sessions, and browser automation can affect the
user's project and host environment. Isolation and security controls are
required for per-conversation review, merge, or discard (worktrees), for
process containment (PTY), and for SSRF/sandbox safety (browser).

## Decision

- **Worktrees**: Create a dedicated Git worktree on a new branch
  (`gaggle/chat-<id>`) for chats in `worktree` mode. All tool execution inside
  that chat uses the worktree path as root. Merge and discard require explicit
  user confirmation and update `worktrees.status`. All lifecycle events are
  recorded in `audit_logs`. Non-Git repositories (`vcs_kind = 'none'`) reject
  worktree creation. Worktree paths are validated via `WorkspacePathPolicy`.
- **PTY**: Terminal sessions use `portable-pty` with process-group termination
  on close, wall-clock and idle timeouts, and chat/session scoping. Sessions
  are tracked and reaped on shutdown.
- **Browser**: Browser automation uses an isolated Chromium profile per session
  with SSRF guards (scheme/host validation, redirect limits, blocked local
  targets), non-HTTP(S) scheme blocking, and session cleanup on shutdown. A
  locally installed Chromium/Chrome binary is required; absence degrades the
  feature rather than faking it.

All three are routed through `PermissionEngine` and subject to path/URL
validation before execution.

## Consequences

- Main project workspace remains untouched until merge.
- Terminal and browser sessions are scoped to a project/chat and closed on
  shutdown.
- Optional capabilities degrade gracefully when their host dependency is
  missing.
- Security tests cover path traversal and symlink escape for worktrees.
