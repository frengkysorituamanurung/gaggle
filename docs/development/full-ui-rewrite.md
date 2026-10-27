# Full UI Rewrite — Gaggle

## Current architecture discovered

- Gaggle is a Rust workspace (axum server + SQLite) serving a React/Vite SPA from `ui/dist`.
- Backend exposes REST/SSE at `/api/*`; static assets at `/assets/*`.
- No desktop framework; browser-based local UI.
- Chat flow: `POST /api/chats/{id}/messages` runs agent synchronously and returns `AgentLoopOutcome`; SSE at `/api/runs/{id}/events`.
- Persistence: SQLite with migrations; projects, chats, messages/parts, runs, approvals, providers/models, extensions, recipes, schedules, memories, settings.

## Existing UI limitations

- Previous scaffold was a single placeholder screen.
- No design system, navigation, or backend integration.

## Actual backend capabilities mapped

| UI operation | Backend capability |
|---|---|
| Create project | POST /api/projects |
| List projects | GET /api/projects |
| Create chat | POST /api/chats |
| Get chat | GET /api/chats/{id} |
| Update chat | PATCH /api/chats/{id} |
| List project chats | GET /api/projects/{id}/chats |
| Get messages | GET /api/chats/{id}/messages |
| Submit message | POST /api/chats/{id}/messages |
| Cancel run | POST /api/runs/{id}/cancel |
| Stream events | GET /api/runs/{id}/events (SSE) |
| Pending approvals | GET /api/approvals/pending |
| Decide approval | POST /api/approvals/{id}/decide |
| Providers/models | GET/POST /api/providers, GET /api/providers/{id}/models |
| Settings | GET/PUT /api/settings |
| Skills | GET /api/skills |
| Extensions inventory | GET /api/extensions/inventory |
| Recipes | GET /api/recipes |
| Schedules | GET /api/schedules |
| Diagnostics | GET /api/diagnostics |

## New UI architecture implemented

- React 19 + Vite + TypeScript SPA in `ui/`.
- Design tokens in `ui/src/styles.css` (light/dark).
- App shell: sidebar navigation + main content (`ui/src/App.tsx`, `Sidebar.tsx`).
- Shared components: Markdown renderer.
- Screens: Chat, History, Skills, Recipes, Extensions, Scheduler, Settings.
- Integration layer: `ui/src/lib/api.ts` typed fetch adapter; `ui/src/lib/store.tsx` app state.
- Chat flow uses real backend; messages/parts rendered; approvals manageable in Settings.

## Rust backend changes

- None required; only formatting fix in `crates/gaggle-ui/src/lib.rs`.

## Database changes

- None.

## Skills used

- ponytail, architecture, gaggle-architecture, rust-engineering, ui-ux, database-sqlite, gaggle-db-schema, agent-runtime, security, testing-qa, observability-reliability, product-manager, research-thinking, skills-and-extensions, provider-integrations, documentation-open-source, performance-cross-platform, git-workflows.

## Verification

- `npm run build` in `ui/`: success (tsc + vite).
- Server smoke: `gaggle serve` serves new index; project/chat/message flow returns completed outcome and persisted messages.
- `cargo fmt --all -- --check`: pass.

## Incomplete features / blockers

- Token-level streaming: backend runs synchronously in submit handler; SSE events are recorded but client receives final outcome only. True incremental streaming would require an async run-start endpoint (backend change).
- Run resumption after approval: backend has no resume endpoint; approvals can be decided but run does not auto-continue.
- Scheduler create/edit not implemented in UI (read-only list).
- Provider model refresh/create-model not wired beyond provider creation.
- No frontend test harness configured.