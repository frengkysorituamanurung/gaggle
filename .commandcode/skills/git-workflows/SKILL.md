---
name: git-workflows
description: Safely inspect repositories, create patches, review diffs, manage branches and worktrees, and protect user changes.
version: 1.0.0
---

# Git Workflow Skill

## Mission
Make agent-generated code changes transparent and recoverable while preserving user-owned work.

## Rules
- Inspect repository status and current branch before mutations.
- Never discard, reset, overwrite, or clean user changes without explicit authorization.
- Distinguish pre-existing changes from changes created by the current run.
- Prefer small, reviewable patches.
- Show changed files, additions/deletions, and relevant diff context.
- Do not automatically commit, push, merge, or rewrite history unless explicitly requested and authorized.
- Treat hooks and repository scripts as executable untrusted code.
- Avoid destructive Git operations by default.
- Validate the target repository and working directory for every operation.

## Worktrees
- Confirm base branch/ref and worktree path.
- Avoid reusing a path owned by another worktree.
- Track lifecycle and cleanup failures.
- Never delete a worktree with uncommitted user changes without explicit authorization.
- Explain merge conflicts and preserve conflict evidence.

## Review
Before reporting completion, inspect the final diff and verify tests actually run. If verification was skipped or failed, state that clearly.
