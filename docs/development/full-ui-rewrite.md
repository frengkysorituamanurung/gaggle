# Gaggle Full UI Rewrite Audit

## Current architecture

- Backend: Rust workspace with `gaggle-server` (Axum), `gaggle-ui` (embedded HTML/CSS/JS), `gaggle-core`, `gaggle-db`, `gaggle-tools`, `gaggle-mcp`, `gaggle-recipes`.
- UI delivery: `crates/gaggle-ui/src/lib.rs::render_index_html()` returns one self-contained HTML document. `index_handler()` in `crates/gaggle-server/src/lib.rs` serves it at `/`.
- Backend API: REST/SSE under `/api/*` (projects, chats, messages, providers, models, settings, extensions, recipes, schedules, memories, approvals, runs, worktrees, diagnostics).
- Persistence: SQLite via `gaggle-db`, migrations, repositories.
- Agent runtime: bounded loop, streaming, approvals, cancellation, checkpoints in `gaggle-core`.
- Desktop packaging: launcher starts Axum server and opens browser/app-mode window. No Electron.

## Current UI limitations

- Single large embedded HTML/JS document (~4,500 lines).
- UI logic is an IIFE with global state and direct fetch calls.
- Styling is hand-written CSS with design tokens in Rust.
- Some controls are partially wired to backend APIs.
- No separate frontend build pipeline or TypeScript project.
- Limited automated UI tests; embedded JS syntax checked by `.github/scripts/check-ui-embedded.sh`.

## Backend capabilities available

- Project/workspace CRUD and files/git/worktrees.
- Chat/session creation, messages, compact, export.
- Provider/model CRUD, OAuth, active model selection.
- Settings, permissions, command rules.
- Extensions/MCP discovery and permissions.
- Recipes, schedules, memories.
- Approvals, runs, cancellation, SSE events.
- Diagnostics export.
- Browser/PTY/worktree tools.

## Integration compatibility gaps

- No typed frontend service layer; UI calls fetch directly.
- No frontend build/test pipeline.
- No component-level tests.
- Some secondary screens are partial.
- Event handling is manual and must be hardened for session switching.

## Proposed target architecture

Keep Gaggle's existing architecture. Improve the embedded HTML UI incrementally:

- Centralize design tokens and reusable CSS classes.
- Add a small typed service layer in the embedded JS.
- Harden event subscription cleanup.
- Complete settings, diagnostics, memory, recipe, schedule, and worktree panels.
- Add browser-level smoke tests for critical flows.
- Keep Rust backend as source of truth.

A full TypeScript/Electron rewrite is explicitly out of scope per the migration plan.

## Implementation phases

1. Audit and compatibility mapping.
2. UI foundation: tokens, layout, shared components.
3. Backend integration hardening.
4. Complete chat workflow.
5. Remaining screens.
6. Stabilization and tests.

## Risk mitigation

- Preserve backend security boundaries.
- Keep changes small and verifiable.
- Run fmt/clippy/tests in CI.
- Do not remove working functionality.

## Testing strategy

- Rust unit/integration tests.
- Embedded JS syntax check.
- Browser smoke tests for critical flows.
- Manual verification of chat, approvals, settings, and persistence.
