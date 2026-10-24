# UI and Backend Integration Audit

This document records the Phase 1 audit and design foundation for Gaggle's desktop
UI/UX and end-to-end backend integration work. It is based on the current
repository state on `development` (`da00963`) and the implementation already
present in the Rust workspace.

## 1. Scope and method

The audit inspected:

- `AGENTS.md` and repository instructions
- `README.md`, `CONTRIBUTING.md`, and files under `docs/development`
- The Rust workspace and all crates under `crates/`
- Database schema and migrations in `crates/gaggle-db`
- Server routes and handlers in `crates/gaggle-server`
- The embedded UI implementation in `crates/gaggle-ui`
- Existing tests under `crates/*/tests`
- GitHub Actions workflows under `.github/workflows`
- Packaging and release scripts under `.github/scripts`

No new runtime behavior was added in this phase. The purpose is to establish an
accurate implementation inventory, design foundation, integration boundaries, and
test baseline before further UI work.

## 2. Repository architecture

Gaggle is a Rust workspace with the following crates:

| Crate | Responsibility |
| :--- | :--- |
| `gaggle-db` | SQLite persistence, migrations, repositories, FTS5 search |
| `gaggle-core` | Domain types, permission engine, agent runtime, run recovery, context assembly, compaction, providers, diagnostics, credentials |
| `gaggle-tools` | Controlled filesystem, shell, memory, web, browser, terminal, diff, and Git tools |
| `gaggle-mcp` | MCP client transports, extension registry, tool limits |
| `gaggle-recipes` | Recipe parsing, parameter resolution, templating, scheduling |
| `gaggle-server` | Axum HTTP/SSE API, orchestration, chat export, UI hosting |
| `gaggle-cli` | CLI, diagnostics, desktop launcher |
| `gaggle-ui` | Embedded HTML/CSS/JavaScript UI rendered by the server |

The UI is not a separate frontend application. `gaggle-ui::render_index_html()`
generates one HTML document containing CSS and JavaScript. The server serves it
from `/`. This keeps the frontend and backend in one deployable Rust binary and
avoids a separate Node.js build.

## 3. Application entry points

- `gaggle` CLI: `crates/gaggle-cli/src/main.rs`
- Desktop launcher: `crates/gaggle-cli/src/bin/gaggle-desktop.rs`
- Server: `crates/gaggle-server/src/lib.rs`
- UI renderer: `crates/gaggle-ui/src/lib.rs`

The desktop launcher starts the existing Axum server and opens the user's
browser. It does not introduce a second server or a native webview.

## 4. Backend API inventory

The server exposes the following route groups. All API paths are served by the
Rust backend and consumed by the embedded UI.

### 4.1 Core and diagnostics

- `GET /api/health`
- `GET /api/diagnostics`
- `GET /api/diagnostics/export`
- `GET /api/settings`
- `PUT /api/settings`

### 4.2 Projects and workspaces

- `GET /api/projects`
- `POST /api/projects`
- `PATCH /api/projects/{id}`
- `GET /api/projects/{id}/chats`
- `GET /api/projects/{id}/files`
- `GET /api/projects/{id}/git-status`
- `GET /api/projects/{id}/diff`
- `GET /api/projects/{id}/worktrees`
- `GET /api/projects/{id}/command-rules`
- `POST /api/projects/{id}/command-rules`
- `GET /api/projects/{id}/tool-permissions`
- `PUT /api/projects/{id}/tool-permissions`

### 4.3 Providers and models

- `GET /api/providers`
- `POST /api/providers`
- `GET /api/providers/{id}/models`
- `POST /api/providers/{id}/models`
- `POST /api/providers/{id}/oauth/chatgpt/start`
- `POST /api/providers/{id}/oauth/chatgpt/exchange`
- `GET /api/oauth/chatgpt/callback`

### 4.4 Memory

- `GET /api/memories`
- `POST /api/memories`
- `GET /api/memories/export`
- `GET /api/memories/{id}`
- `PATCH /api/memories/{id}`
- `PUT /api/memories/{id}`
- `DELETE /api/memories/{id}`

### 4.5 Extensions and MCP

- `GET /api/extensions`
- `POST /api/extensions`
- `GET /api/extensions/inventory`
- `PATCH /api/extensions/{id}`
- `DELETE /api/extensions/{id}`
- `POST /api/extensions/{id}/discover`
- `GET /api/extensions/{id}/permissions`
- `POST /api/extensions/{id}/permissions`

### 4.6 Recipes and schedules

- `GET /api/recipes`
- `POST /api/recipes`
- `GET /api/recipes/{id}`
- `POST /api/recipes/{id}/execute`
- `GET /api/schedules`
- `POST /api/schedules`
- `GET /api/schedules/{id}`
- `PATCH /api/schedules/{id}`
- `DELETE /api/schedules/{id}`

### 4.7 Chats, runs, and approvals

- `POST /api/chats`
- `POST /api/chats/{id}/messages`
- `POST /api/chats/{id}/compact`
- `PATCH /api/chats/{id}`
- `GET /api/chats/{id}/export`
- `GET /api/runs/{id}/events`
- `POST /api/runs/{id}/cancel`
- `GET /api/approvals/pending`
- `POST /api/approvals/{id}/decide`

### 4.8 Worktrees, terminal, and browser

- `GET /api/worktrees/{id}/git-status`
- `GET /api/worktrees/{id}/diff`
- `POST /api/worktrees/{id}/merge`
- `POST /api/worktrees/{id}/discard`
- Terminal routes are mounted through `terminal_routes()`
- Browser actions are executed through `browser.*` built-in tools and persisted in
  `browser_sessions` and `browser_actions`

## 5. Persistence and data model

The database is SQLite with WAL mode, foreign keys, and versioned SQLx
migrations. Primary keys are UUIDv7 text values and timestamps are Unix epoch
milliseconds.

Major persisted domains include:

- Projects, chats, messages, message parts
- Runs, run steps, tool calls, approvals
- Worktrees, file changes
- Memories and memory tags
- Providers and models
- Extensions, extension tools, chat extensions, extension permissions
- Recipes, recipe parameters, recipe sub-recipes, recipe extensions
- Schedules
- Terminal sessions
- Browser sessions and browser actions
- Chat summaries and compaction metadata
- Command rules, tool permissions, settings, audit logs

The UI must use existing repositories and must not access SQLite directly.

## 6. Agent runtime and provider integration

The agent runtime supports:

- Bounded agent loop with step limits
- Native provider streaming
- Tool-call orchestration
- Permission enforcement
- Run persistence and recovery
- Cancellation
- Context assembly
- Context compaction
- Subagent spawning with bounded concurrency
- Tool execution status and approval workflows

Provider adapters include Anthropic, Ollama, and ChatGPT OAuth. Provider URLs are
validated against SSRF and credential leakage risks.

## 7. Tools, permissions, and safety

Tool execution is routed through `PermissionEngine`. Permission modes include
`ask`, `smart`, `full`, and `chat_only`. Deny rules are authoritative.

Built-in and extension tools include filesystem, shell, Git, worktree, memory,
web, browser, terminal, recipe, and MCP capabilities. Approval binding is tied
to exact tool arguments and command fingerprints.

Security rules require:

- Canonicalized project roots
- Traversal and symlink-escape prevention
- Secret redaction
- No plaintext secrets in SQLite, logs, or UI
- OS credential store usage through `secret_ref`
- No `pull_request_target` execution of untrusted code

## 8. Skills, recipes, extensions, and scheduling

The repository supports:

- Skill and extension metadata
- MCP stdio, streamable HTTP, and SSE transports
- Extension lifecycle and capability grants
- Recipe parsing from inline JSON/YAML, files, and URLs
- Parameter validation and template rendering
- Recipe execution and sub-recipes
- Cron scheduling with timezone handling and missed-run catch-up
- Subagent execution with inherited permissions

These capabilities are primarily API/backend driven and require UI surfaces in
later phases.

## 9. Terminal and browser automation

Terminal sessions use PTYs with in-memory scrollback and persisted metadata.
Browser automation uses headless Chromium with isolated profiles, recorded
actions, screenshot attachments, and SSRF/scheme validation.

Both are cross-platform concerns and require availability diagnostics on
macOS and Linux.

## 10. Observability and diagnostics

The codebase includes:

- Structured tracing
- Correlation IDs for chat, run, and tool call
- Redacted diagnostics export
- `gaggle doctor`
- Actionable error messages
- Settings diagnostics UI hooks

No automatic telemetry or crash reporting is implemented or permitted.

## 11. Data portability

The CLI supports:

- Chat export/import
- Memory export
- Database backup
- Database restore with checksum verification

These operations are cross-platform and must remain consistent with SQLite
migration and attachment storage rules.

## 12. CI/CD and packaging

The repository has:

- CI quality gates on `development` and `main`
- Native macOS and Linux package builds
- Pull-request package builds
- Tag-only stable releases from commits on `main`
- SHA-256 checksums and build manifests
- Optional macOS signing and notarization
- CodeQL and Dependabot

Package builds are artifact-only and do not publish stable releases.

## 13. Capability inventory

### 13.1 Fully implemented

- Project registration and workspace policy
- Provider and model configuration
- Memory CRUD, search, tags, and export
- Chat and message persistence
- Agent loop with streaming and tool execution
- Permission engine and approval workflows
- Context assembly and compaction
- Subagent spawning
- MCP client transports and extension lifecycle
- Recipe parsing, execution, and scheduling
- Terminal sessions
- Browser automation
- Observability and diagnostics
- Export, import, backup, and restore
- Native CI/CD packaging

### 13.2 Partially implemented

- Desktop UI shell and screens
- Settings UI coverage
- Secondary screens for skills, recipes, extensions, scheduler, and history
- Theme switching persistence
- Tool execution visualization
- Streaming UX polish
- Keyboard shortcut coverage

### 13.3 Backend functionality without UI

- Many provider, memory, extension, recipe, schedule, terminal, browser, and
  diagnostic APIs have backend support but limited or no dedicated UI.
- Export/import and backup/restore are CLI-first.
- Subagent and compaction behavior is runtime-driven.

### 13.4 UI functionality without complete backend integration

- Some UI controls are presentation-level and must be connected to real backend
  state in later phases.
- Any placeholder or mock-like state must be removed or clearly marked.

### 13.5 Missing functionality

- Full end-to-end UI integration for all secondary screens
- Comprehensive UI tests for all screens
- Native end-to-end desktop launch tests in CI
- Linux runtime verification for all packages
- Windows support

### 13.6 Functionality requiring additional architectural work

- Native desktop shell beyond browser-based UI
- Real-time multi-user or remote access
- Cloud backup or sync
- Native webview replacement

## 14. Current UI implementation

The UI is implemented in `crates/gaggle-ui/src/lib.rs` as a server-rendered HTML
document with embedded CSS and JavaScript. It includes:

- Design token structs and light/dark token values
- CSS custom property generation
- App shell with sidebar, main workspace, and details panel
- Responsive drawer behavior
- Conversation rendering
- Composer
- Markdown and code-block rendering
- Streaming and smart scroll behavior
- Settings modal with six tabs
- Provider/model management
- Permission and command-rule management
- Extensions, memory, and diagnostics panels
- Onboarding card
- Quick switcher
- Keyboard shortcuts and focus management
- Modal landmarks and ARIA attributes
- XSS-safe DOM construction

The UI communicates with the backend through `/api/*` endpoints using
`fetch` and `EventSource`.

## 15. Design system foundation

### 15.1 Existing tokens

The current design token system is defined in `crates/gaggle-ui/src/lib.rs`:

- `DesignTokens`
- `ColorTokens`
- `TypographyTokens`
- `SpacingTokens`
- `BorderTokens`
- `MotionTokens`

Light and dark themes are generated through `format_css_variables()` and injected
into the rendered HTML.

### 15.2 Semantic token baseline

The current token set covers:

- Application, panel, and surface backgrounds
- Primary and muted text
- Accent, danger, and success colors
- Border color
- Font families and sizes
- Spacing scale
- Border radius and width
- Motion durations and easing

Later phases should extend this set with explicit semantic tokens for:

- Sidebar background
- Elevated surface
- Secondary text
- Focus ring
- Warning
- Information
- Divider
- Overlay backdrop

Any token additions must update both `DesignTokens` and `format_css_variables()`
and must be covered by the existing CSS variable completeness tests.

### 15.3 Reusable primitives

The UI already uses reusable conceptual primitives through CSS classes and
JavaScript helpers:

- `createEl` for safe DOM construction
- App shell panels
- Modal overlays
- Settings tabs and panels
- Status banners
- Empty states
- Form controls
- Code blocks and copy buttons
- Focus trapping helpers

Future work should preserve these primitives rather than replacing them with
ad hoc markup.

## 16. Integration boundaries

The frontend must remain a presentation layer. The established boundaries are:

1. UI calls backend HTTP/SSE endpoints only.
2. Database access occurs through Rust repositories.
3. Permission enforcement remains in the backend.
4. Provider credentials remain in the OS credential store through `secret_ref`.
5. Agent execution and tool calls remain in `gaggle-core` and `gaggle-tools`.
6. UI state must not become a second source of truth for conversations.
7. Streaming events must be tied to real run IDs and cleaned up on navigation.
8. Errors must propagate from backend to UI as actionable messages.

## 17. Testing baseline

The repository's standard quality gates are:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
bash -n .github/scripts/*.sh
.github/scripts/check-ui-embedded.sh
```

The UI crate has tests for:

- Design token CSS variable completeness
- XSS-safe HTML generation
- Workspace switcher and project modal
- Responsive app shell and connection banner
- Settings, onboarding, providers, permissions, extensions, memory, diagnostics
- Conversation streaming and code rendering
- Keyboard shortcuts and focus trapping
- Modal landmarks and ARIA attributes

Backend tests cover schema, repositories, providers, context assembly, agent
runtime, permissions, tools, worktrees, and API endpoints.

CI runs quality gates and native package builds on `development` and `main`.
Package builds are artifact-only and do not publish stable releases.

## 18. Known limitations

- The UI is browser-based and served by the local Rust server.
- There is no native desktop webview in this phase.
- Some secondary screens are only partially connected to backend capabilities.
- Linux package runtime verification depends on GitHub-hosted runners.
- macOS signing and notarization are optional and secrets-gated.
- No Windows target is implemented.
- Some UI tests verify rendered HTML rather than full browser interaction.

## 19. Recommended next steps

1. Extend design tokens with explicit semantic colors for warning, information,
   focus, elevated surfaces, and secondary text.
2. Connect all secondary screens to real backend APIs.
3. Add browser-level or DOM interaction tests for critical chat flows.
4. Verify streaming cancellation against the real runtime.
5. Add Linux runtime smoke tests for terminal and browser automation.
6. Expand accessibility tests for dialogs and keyboard-only workflows.
7. Keep CI package builds as the deployment verification path.

## 20. Phase 1 completion statement

Phase 1 is complete when this audit document exists and accurately describes the
current architecture, capability inventory, design foundation, integration
boundaries, testing baseline, and known limitations. This document satisfies that
deliverable and provides the basis for Phase 2 implementation.
