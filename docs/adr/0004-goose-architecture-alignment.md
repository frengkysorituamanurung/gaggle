# ADR 0004 — Align Gaggle with Goose Architecture

- Status: Accepted
- Date: 2026-10-29
- Reference: local `goose/` checkout at `775b3958c`

## Context

Gaggle is a Rust workspace with eight crates, a durable SQLite run model, an
Axum API, and a React UI. Goose is a larger Rust workspace organized around
`goose-agent` operations/state machines, provider and MCP crates, session
services, and an Electron desktop app with ACP backend. The requested target
is architectural and behavioral parity with Goose, not a one-off feature port.
Gaggle's desktop shell will use Electron with ACP backend, matching Goose's
architecture. Tauri remains as a fallback option.

Replacing Gaggle's workspace wholesale would also replace its persistence and
application contracts. In particular, Gaggle's `runs`, ordered `run_events`,
checkpoints, project roots, and approval records are the system of record and
must not be silently discarded or replayed as if they were Goose sessions.

## Options considered

1. **Replace Gaggle with the Goose checkout.** Fastest route to Goose behavior,
   but drops Gaggle-specific project/run semantics, risks existing user data,
   and leaves Gaggle's product and packaging unspecified.
2. **Port Goose's architecture in stages, behind Gaggle adapters.** More work,
   but permits state-machine and desktop parity while preserving data,
   permission boundaries, and a reversible migration path.
3. **Keep Gaggle's current architecture and continue feature-by-feature ports.**
   Lowest migration risk, but does not meet the requested architecture change.

## Decision

Choose option 2. Use the pinned Goose checkout as a read-only architectural
reference and migrate the runtime toward Goose's operation/state-machine model,
provider/MCP boundaries, and desktop experience. Do not make Gaggle depend on
the nested checkout or copy its workspace over Gaggle in one destructive step.

Keep Gaggle's SQLite database authoritative during migration. Implement a
Gaggle persistence adapter for the new runtime so effects, run events, and
checkpoints are committed before continuing past side effects. Preserve
Gaggle's permission engine as a server-side enforcement boundary; Goose-style
operations must not bypass deny rules or tool approvals. Migrate providers,
extensions, and UI through explicit compatibility layers and versioned data
migrations.

Any code ported from Goose must retain its Apache-2.0 notices and attribution.
The nested checkout remains read-only and must not be altered by this work.

## Consequences

- The rewrite is a sequence of independently verifiable phases, not a single
  file-copy operation.
- Public CLI/API behavior may change only with compatibility or migration notes.
- Existing projects, chats, messages, approvals, and runs remain readable during
  the transition; no schema is removed until an export/upgrade path is tested.
- Goose features that conflict with Gaggle's durable-run or authorization
  invariants require an adapter or are excluded, rather than weakening those
  invariants for superficial parity.

## Security and data impact

- Re-check authorization immediately before every tool side effect, including
  after approval and after recovery.
- Store only credential references in SQLite; never import plaintext secrets
  from Goose configuration into Gaggle storage.
- Do not retry a side effect after a crash unless its outcome is known or the
  operation is idempotent.
- Back up the existing database before every schema transition and test upgrades
  from representative Gaggle versions.

## Migration and validation

The phases and exit criteria are maintained in
[`../development/goose-architecture-rewrite-plan.md`](../development/goose-architecture-rewrite-plan.md).
Each phase requires focused tests and the quality gates configured in project
CI. The old behavior-port plan is retained as historical context but is
superseded by the architecture rewrite plan.
