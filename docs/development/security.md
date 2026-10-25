# Security Threat Model

This document is the threat model for Gaggle's local-first AI agent runtime. It
complements `SECURITY.md` (disclosure policy) and the non-negotiable constraints
in `AGENTS.md`.

## Scope

In scope:
- Local `gaggle-server` HTTP/SSE API and embedded UI.
- Agent runtime, tool execution, MCP extensions, terminal PTY, browser sandbox.
- SQLite persistence (`gaggle.db`) and OS credential store references.
- Git worktree isolation and project filesystem access.

Out of scope:
- Physical access to an unlocked workstation.
- Compromise of the host operating system or browser engine.
- Network-level attacks against third-party model providers (TLS delegated to provider).

## Assets

1. Source code and project files under registered project roots.
2. Conversation history, run checkpoints, tool calls, and diffs in SQLite.
3. Secrets referenced by `secret_ref` (API keys, OAuth tokens) in the OS
   credential store.
4. Shell and terminal session access (PTY).
5. Browser profile data and screenshots.
6. Audit trail integrity (`audit_logs`).

## Trust boundaries

- **LLM output is untrusted.** The model proposes tool calls but never
  authorizes them. Every call is validated server-side by `PermissionEngine`.
- **MCP extensions are untrusted** until explicitly enabled and granted
  capabilities. They run as separate processes/connections with controlled env.
- **User input** (chat messages, UI actions) is trusted but still validated.
- **Filesystem access** is confined to canonicalized project roots.
- **Terminal/browser sessions** are scoped to a project/chat and closed on
  shutdown.

## Threats and mitigations

### T1: Prompt injection leading to unauthorized tool use
- **Threat**: Malicious content in files, web pages, or tool results instructs
  the model to run destructive commands.
- **Mitigation**: Default permission mode is `ask`; `command_rules` with
  `action = 'deny'` are authoritative even in `full` mode; approvals are
  verified immediately before execution (`verify_approved_execution`); tool calls
  are re-validated server-side.

### T2: Path traversal and symlink escape
- **Threat**: `..` segments or symlinked paths escape the project root.
- **Mitigation**: `WorkspacePathPolicy` canonicalizes and validates all paths;
  filesystem tools reject paths outside the canonical root; worktree paths are
  validated before use.

### T3: Secret leakage
- **Threat**: API keys or tokens stored in SQLite, config, or logs.
- **Mitigation**: Only `secret_ref` identifiers are persisted; plaintext secrets
  live in the OS credential store; structured tracing redacts secret fields;
  exports exclude secret material.

### T4: Shell injection and uncontrolled process execution
- **Threat**: Shell commands execute arbitrary code or spawn orphaned
  processes.
- **Mitigation**: Argument-based `Command::new` (no shell interpolation) where
  possible; sanitized environment; wall-clock and idle timeouts; process groups
  terminated on close; PTY sessions tracked and reaped on shutdown.

### T5: SSRF via MCP HTTP/SSE transports or browser
- **Threat**: Malicious extension URL or navigated URL reaches internal
  services.
- **Mitigation**: Scheme/host validation, redirect limits, destination scope
  checks for MCP transports; browser navigation blocks non-HTTP(S) schemes and
  local-only targets where configured.

### T6: Cross-chat data leakage
- **Threat**: Tool execution or browser/terminal sessions operate on another
  chat's workspace.
- **Mitigation**: Worktree isolation per chat; tool routing uses chat-scoped
  root paths; browser/terminal sessions are keyed by chat/session id; active
  tool limit (<= 25) bounds blast radius.

### T7: Audit tampering or gaps
- **Threat**: Lifecycle events (approvals, worktrees, extensions, terminal/browser
  sessions) are not recorded or can be altered.
- **Mitigation**: Append-only `audit_logs` table written in the same transaction
  as the state change; repository methods record lifecycle events; logs are
  readable via `list_audit_logs`.

### T8: Run recovery side effects
- **Threat**: Replaying a run re-executes non-idempotent tool calls.
- **Mitigation**: Recovery resumes from the latest checkpoint; non-idempotent
  side effects are not automatically retried; cancellation registry prevents
  duplicate in-flight execution.

## Residual risks

- OS keychain integration is not fully wired; an in-memory credential store is
  used in some paths.
- Browser requires a local Chromium binary; absence degrades the feature rather
  than faking it.
- SQLite single-writer contention can surface as `SQLITE_BUSY` under heavy
  concurrent writes.
- Audit logs have no retention, export, or tamper-evidence policy yet.

## Verification

Security-sensitive changes must include tests for path traversal, permission
denial, and audit recording, and must pass `cargo clippy --workspace
--all-targets -- -D warnings` and `cargo test --workspace`.
