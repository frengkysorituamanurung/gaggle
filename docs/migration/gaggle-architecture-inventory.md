# Gaggle Architecture Inventory

Source: `/Users/frengky/Documents/projects/gaggle`.

## Workspace
- Rust workspace (`Cargo.toml`, crates/): gaggle-db, gaggle-core, gaggle-tools, gaggle-mcp, gaggle-recipes, gaggle-server, gaggle-cli, gaggle-ui (placeholder).
- Desktop wrapper: `desktop/` (Tauri shell, `desktop-app/src-tauri`).
- Frontend: `ui/` (React + TS + Vite + Tailwind v4), served by gaggle-server from `ui/dist`.

## Frontend
- Entry: `ui/src/main.tsx`, `ui/src/App.tsx` (flat routes, AppProvider + NavigationProvider).
- State: `ui/src/lib/store.tsx` (AppProvider), `NavigationContext`.
- Backend client: `ui/src/lib/api.ts` (REST under /api/*), run events via `ui/src/lib/runEvents.ts` (SSE).
- Screens: Chat, Hub, History, Skills, Recipes (+manage), Extensions (+manage), Scheduler, Settings, Memories (+manage), Permissions, Workspace, Terminal, Diagnostics, Worktrees, Context, Pair, ConfigureProviders, Onboarding, Apps, StandaloneApp, Launcher.
- Components: many goose-mirrored primitives and cards.

## Backend
- Axum HTTP/SSE API in gaggle-server; endpoints /api/*, /api/runs/{id}/events.
- Agent runtime (RunEngine) server-side; approvals, permissions, tools, worktrees, terminal PTY, MCP extensions, recipes, schedules, memories, skills.
- SQLite persistence (gaggle-db), UUIDv7, migrations, FTS5.

## Desktop
- Tauri shell; local static server; asset loading; packaging scripts. macOS ZIP with Gaggle.app, Linux bundles.

## Build
- Frontend: `npm run build` (tsc -b && vite build) in ui/.
- Rust: cargo build/test; checks via cargo fmt/clippy.