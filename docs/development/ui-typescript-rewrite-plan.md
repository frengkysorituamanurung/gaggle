# Gaggle UI TypeScript Rewrite Plan

Status: proposed. This plan supersedes the UI section of
`docs/development/goose-to-gaggle-migration-plan.md`.

## Goal

Replace Gaggle's single embedded HTML document with a TypeScript/React frontend
based on Goose's desktop UI, while keeping Gaggle's Rust backend and SQLite
database as the system of record.

## Scope

- Port Goose desktop renderer components, styles, and interaction patterns.
- Replace Electron IPC calls with calls to Gaggle's HTTP/SSE API.
- Add a frontend build step to CI and packaging.
- Keep Rust crates as backend; do not port Goose's Rust agent.

## Out of scope

- Electron main process, tray, auto-updater, native menus.
- Windows support.
- Changing Gaggle's permission, storage, or agent architecture.

## Target architecture

- `ui/`: Vite + React + TypeScript frontend copied/adapted from
  `goose/ui/desktop`.
- `crates/gaggle-server`: serve built assets from `ui/dist`, with SPA fallback
  to `index.html`; keep `/api/*` and `/` behavior compatible.
- `crates/gaggle-ui`: reduce to design tokens/CSS helpers or remove if no
  longer needed.
- CI: install Node/pnpm, build UI, run typecheck/lint/tests, embed or attach
  built assets.
- Packaging: include `ui/dist` in macOS/Linux bundles.

## Phase 0 - Inventory and adapter boundary

1. List all `window.electron.*` calls in Goose UI.
2. Map each call to an existing Gaggle API endpoint or mark as new work.
3. Define a small `gaggleClient.ts` adapter instead of touching every component.

## Phase 1 - Frontend skeleton

1. Copy Goose renderer source into `ui/`.
2. Remove Electron-only entry points and main process.
3. Add Vite web build producing `ui/dist`.
4. Add a `window.gaggle` shim backed by `fetch`/`EventSource`.

## Phase 2 - API adapter

1. Implement project, chat, message, provider, settings, approvals, and run
   event calls against Gaggle routes.
2. Keep Goose component APIs stable behind the adapter.

## Phase 3 - Feature parity

1. Chat streaming and approvals.
2. Projects/worktrees/files/git status.
3. Providers/models/settings.
4. Memory, recipes, schedules.
5. Diagnostics and onboarding.

## Phase 4 - Cutover and cleanup

1. Server serves `ui/dist`.
2. Remove embedded HTML renderer or keep as fallback.
3. Update packaging scripts.
4. Update docs and known limitations.

## Verification

- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- UI typecheck, lint, and unit tests in CI
- Packaging smoke test for macOS and Linux

## Risks

- Large surface area and long migration.
- API mismatch between Goose IPC and Gaggle REST/SSE.
- Packaging must include built frontend assets.
- No local build/test per repository policy; all verification runs in CI.
