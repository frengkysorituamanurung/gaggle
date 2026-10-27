---
name: rust-engineering
description: Implement maintainable, safe, idiomatic Rust for Gaggle's core, services, tools, and platform integrations.
version: 1.0.0
---

# Rust Engineering Skill

## Mission
Produce reliable, idiomatic Rust with clear boundaries, predictable error handling, bounded concurrency, and testable behavior.

## Architecture boundaries
Keep domain logic independent from UI and platform-specific code. Prefer explicit interfaces between:
- Domain and application use cases.
- Agent runtime and model-provider adapters.
- Tool registry and tool implementations.
- Permission/approval services.
- Persistence/repositories.
- Process execution and sandboxing.
- API transport and UI.

Follow the repository's established crate structure unless an ADR approves a change.

## Implementation rules
- Inspect nearby code and conventions before editing.
- Prefer typed domain models and explicit enums for lifecycle states.
- Use `Result` and meaningful error types; avoid panics in normal runtime paths.
- Avoid `.unwrap()`/`.expect()` on user-controlled input, I/O, network, database, and process operations.
- Keep async executor threads free of blocking work; use appropriate blocking boundaries.
- Bound spawned tasks, queues, output buffers, and concurrent tool calls.
- Propagate cancellation and deadlines.
- Avoid unnecessary shared mutable state and lock scopes that cross `.await`.
- Validate untrusted data at system boundaries.
- Keep dependencies minimal and check maintenance, license, and platform support.
- Do not suppress compiler warnings without a documented reason.

## Provider and streaming code
- Normalize provider-specific behavior behind adapters.
- Handle timeouts, cancellation, rate limits, malformed responses, and partial streams.
- Do not assume all models support tools, vision, or reasoning.
- Do not log raw prompts, credentials, or sensitive tool output by default.
- Make retries explicit and only retry operations known to be safe.

## Tool execution
- Use typed tool definitions and validated input schemas.
- Route every tool through the central policy engine.
- Enforce timeouts, output caps, cancellation, and structured errors.
- Keep side effects explicit and auditable.
- Do not execute model-generated shell text by concatenating it into a command string.

## Workflow
1. Inspect repository and affected crate APIs.
2. Describe the smallest implementation approach.
3. Implement incrementally.
4. Add unit and integration tests for success, error, timeout, and cancellation paths.
5. Run formatting, linting, and relevant tests.
6. Review for panic paths, races, resource leaks, and secret exposure.
7. Summarize files changed and actual validation results.

## Quality gates
Use the repository's configured commands. Common checks may include `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo test --workspace`; do not claim these passed unless executed successfully.
