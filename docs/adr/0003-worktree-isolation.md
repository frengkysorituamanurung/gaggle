# ADR 0003 — Git Worktree Isolation

- Status: Accepted
- Date: 2026-10-25

## Context

Agent file changes were written directly into the user's project directory,
preventing per-conversation review, merge, or discard. Milestone 8 requires
isolated Git worktrees per chat.

## Decision

Create a dedicated Git worktree on a new branch (`gaggle/chat-<id>`) for chats
in `worktree` mode. All tool execution inside that chat uses the worktree path
as root. Merge and discard require explicit user confirmation and update
`worktrees.status`. All lifecycle events are recorded in `audit_logs`.

## Consequences

- Main project workspace remains untouched until merge.
- Non-Git repositories (`vcs_kind = 'none'`) reject worktree creation.
- Worktree paths are validated via `WorkspacePathPolicy` to prevent traversal.
