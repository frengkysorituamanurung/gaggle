---
name: architecture
description: Make and review Gaggle architecture decisions with emphasis on clear boundaries, durable execution, local-first storage, and cross-platform support.
version: 1.0.0
---

# Architecture Skill

## Mission
Keep Gaggle modular and understandable while supporting reliable local AI-agent execution on macOS and Linux.

## Principles
- Local-first: local project metadata and history remain on-device unless the user explicitly configures an external service.
- One authoritative source of truth for run history.
- Explicit lifecycle state machines.
- Least privilege and defense in depth.
- UI-independent domain logic.
- Stable interfaces around providers, tools, storage, and platform-specific facilities.
- Prefer proven simple designs over speculative distributed architecture.

## Before changing architecture
1. Inspect the existing implementation and constraints.
2. Describe the problem and affected components.
3. Compare at least two viable approaches for significant decisions.
4. Consider failure modes, migration costs, security, observability, testability, and platform support.
5. Record the decision and rejected alternatives in an ADR.
6. Define a migration path that preserves existing user data.

## Durable run requirements
- Persist run and tool-call intent before side effects where practical.
- Record ordered events and validate state transitions.
- Use transactions for related database mutations.
- Do not assume retrying a process or external request is safe.
- On crash recovery, distinguish known failure from unknown outcome.
- Reconcile uncertain side effects before retrying.
- Treat approval as a stateful, expiring authorization tied to an exact operation.

## SQLite requirements
- Use SQLx migrations and test upgrades from prior schema versions.
- Enable and verify foreign-key enforcement.
- Use transactions for multi-step state changes.
- Add indexes based on query patterns.
- Define backup, restore, retention, and corruption-recovery behavior.
- Keep large binary data and large execution output out of ordinary relational rows where practical.
- Avoid circular foreign-key designs unless insertion and deletion behavior is explicitly tested.

## Cross-platform requirements
Evaluate credential stores, shell and PTY behavior, process-tree cancellation, file watchers, symlinks, paths, WebView/runtime dependencies, packaging, and supported CPU architectures.

## Output
For each significant decision, provide context, options, decision, consequences, security impact, and migration/testing requirements.
