# Known Limitations

This document records capabilities that are intentionally limited, unavailable, or
planned for future work in Gaggle. It complements the feature documentation and
the UI/backend integration audit.

## Credentials and secrets

- API keys and OAuth tokens are stored via an in-memory `CredentialStore`
  abstraction. OS keychain integration is planned but not yet wired.
- The database stores only `secret_ref` identifiers, never plaintext secrets.

## Optional capabilities

- The browser tool requires a locally installed Chromium/Chrome binary. When it
  cannot launch, browser-dependent tests and features are skipped or reported
  unavailable rather than faked.
- PTY terminal sessions depend on platform process-tree and signal behavior;
  behavior may differ between macOS and Linux.
- MCP extensions are treated as untrusted until explicitly enabled and
  authorized. Transport support is limited to implemented and tested transports.

## UI and secondary screens

- Skills and Apps screens are intentionally honest unavailable/planned states.
  Skills are file-based instruction packs loaded from `.opencode/skills`; a
  managed browser is planned. Apps/external application integrations are not yet
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