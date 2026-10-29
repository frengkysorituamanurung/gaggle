# Known Limitations

This document records capabilities that are intentionally limited, unavailable, or
planned for future work in Gaggle. It complements the feature documentation and
the UI/backend integration audit.

## Credentials and secrets

- API keys and OAuth tokens are stored in the OS credential vault (macOS Keychain / Linux Secret Service) through `KeyringCredentialStore`; an in-memory fallback remains for tests and unsupported platforms.
- Desktop: Electron is the primary desktop shell (rebranded from Goose). Tauri is deprecated and will be removed.
- ACP backend: WebSocket JSON-RPC at /acp with all core methods. Streaming notifications and permission bridging implemented.
- UI screens added: Onboarding, Diagnostics, Worktrees, Context Management, Memory Management, Recipe Management, Extension Management, Quick Launcher (Ctrl+K).
- Desktop: Electron is the primary desktop shell. Native menu localization exists (Chinese). ACP backend provides WebSocket JSON-RPC at /acp with all core methods. Tauri is deprecated.
- ACP backend integration: WebSocket JSON-RPC handler at /acp with initialize, session/new, session/prompt (delegated to agent loop with streaming text deltas and tool call notifications), session/cancel, session/list, session/load, session/delete, session/close, session/fork, session/setConfigOption (with config persistence) methods. Usage update notifications sent after completion. Permission request bridging with oneshot channel correlation - waits for client response with 30s timeout. Auto-deny on timeout.
- Desktop: Electron is the primary desktop shell. Native menu localization exists (Chinese). ACP backend provides WebSocket JSON-RPC at /acp with all core methods. Tauri is deprecated.
- UI: Onboarding, Diagnostics, Worktrees, Context Management, Memory Management, Recipe Management, Extension Management, Quick Launcher (Ctrl+K) added.
- Settings: Theme, Keyboard Shortcuts, Live Voice, Dictation sections added.
- McpApps renderer with iframe sandbox and message passing.
- Native menu localization: Chinese (existing). Multi-locale menu translation framework added (i18n-menu.ts). Full translation files for all 16 locales pending.
- ACP-based session management: WebSocket JSON-RPC with all core methods (initialize, session/new, session/prompt, session/cancel, session/list, session/load, session/delete, session/close, session/fork, session/setConfigOption). Streaming notifications and permission bridging implemented. Full HandleDispatchFrom trait parity pending (requires porting 4000+ lines from Goose).
- Provider adapters: Anthropic, Ollama, OpenAI-compatible, Google Gemini, Azure, Databricks, Snowflake are supported. ChatGPT OAuth, local inference, and additional providers are planned.
- Goose session import preserves usage and metadata in redacted artifact tables; credentials are not copied unless `--import-goose-credentials` is used.
- UI supports English, Indonesian, and Japanese locales; full Goose i18n parity is in progress.
  abstraction. OS keychain integration is planned but not yet wired.
- The database stores only `secret_ref` identifiers, never plaintext secrets.

## Optional capabilities

- The browser tool requires a locally installed Chromium/Chrome binary. When it
  cannot launch, browser-dependent tests and features are skipped or reported
  unavailable rather than faked. Browser sessions are closed on server shutdown
  and their temporary profiles are removed.
- PTY terminal sessions depend on platform process-tree and signal behavior;
  behavior may differ between macOS and Linux. Process groups are terminated
  on close to avoid orphaned children.
- MCP extensions are treated as untrusted until explicitly enabled and
  authorized. Transport support is limited to implemented and tested transports.

## UI and secondary screens

- Skills and Apps screens are intentionally honest unavailable/planned states.
  Skills are file-based instruction packs loaded from `.opencode/skills`; a managed
  browser lists discovered skills, while install/uninstall and Apps integrations
  are planned. Apps/external application integrations are not yet
  supported by the backend.
- The UI is a single embedded HTML document rendered by the Rust server. There
  is no separate frontend build or hot-reload workflow.
- Light and dark themes are supported via design tokens and `prefers-color-scheme`.

## Data and concurrency

- SQLite is the single source of truth with WAL mode and a single writer.
  `busy_timeout` is set to 5000 ms; heavy concurrent write contention may
  surface as `SQLITE_BUSY`.
- Backups are full backups only (no incremental backups or cloud sync).
- Chat export/import uses JSON bundles; large histories are bounded by the
  500-message retrieval limit in the hydration endpoint.

## Agent runtime

- Active tools per chat are limited to the recommended maximum of 25.
- Run recovery resumes from the latest checkpoint; non-idempotent side effects
  are not automatically retried.
- Cancellation reaches in-flight execution only when a run is registered in the
  shared `ActiveRunRegistry`.

## Accessibility

- Keyboard navigation, focus indicators, and ARIA landmarks are implemented for
  the core shell, dialogs, and settings. Secondary screens reuse the same
  patterns but have not undergone a full assistive-technology audit.

## Cross-platform

- macOS (Apple Silicon and Intel) and Linux are the supported targets. Windows
  is not supported.
- Credential store, shell discovery, and file watcher behavior are
  platform-dependent and documented per platform where applicable.


## Audit and traceability

- Audit logs are append-only SQLite records written in the same transaction as
  the state change. There is no retention, export, or tamper-evidence policy
  yet.
- Lifecycle events for terminal and browser sessions are recorded, but
  higher-level run/compaction audit coverage is still expanding.

## Architecture decisions

- ADRs for the agent state machine (`0001-agent-state-machine.md`), provider
  trait (`0002-provider-trait.md`), and worktree/PTY/browser
  (`0003-worktree-pty-browser.md`) are recorded under `docs/adr/`. Some
  referenced refactors (state machine operation pipeline) are planned rather
  than fully implemented.
