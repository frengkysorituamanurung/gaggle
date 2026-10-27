# Gaggle Full UI Rewrite - Phase Plan

## Phase A - Audit and compatibility mapping

- Audit crate structure, routes, repositories, runtime, and packaging.
- Map every UI operation to a backend capability.
- Document missing capabilities and blockers.

## Phase B - UI foundation

- Centralize design tokens in `gaggle-ui`.
- Refine application shell, navigation, sidebar, and layout.
- Add reusable CSS classes for panels, dialogs, buttons, forms, badges.
- Add responsive behavior for narrow windows.

## Phase C - Backend integration hardening

- Add a small typed service layer in the embedded JS.
- Normalize fetch errors and loading states.
- Harden SSE subscription cleanup.
- Prevent stale requests from overwriting active sessions.

## Phase D - Complete chat workflow

- Composer, streaming, cancellation, approvals.
- Message rendering and code blocks.
- Session lifecycle and history restoration.
- Tool execution status and errors.

## Phase E - Remaining screens

- Settings and providers.
- Skills, recipes, apps/extensions.
- Scheduler.
- Session history.
- Diagnostics.

## Phase F - Stabilization

- Fix regressions.
- Run fmt, clippy, tests, embedded UI syntax check.
- Document limitations and verify application startup.
