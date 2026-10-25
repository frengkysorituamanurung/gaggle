# ADR 0001 — Agent Runtime State Machine Refactor

- Status: Proposed
- Date: 2026-10-25

## Context

Gaggle's agent runtime (`RunEngine` / `AgentRuntime`) is durable and
checkpointed, but its turn loop is monolithic compared to Goose's discrete
operation pipeline. Phase A of the migration plan proposes refactoring
`AgentRuntime::run_turn_inner` into discrete operations (LLM inference, tool
calling, approval, compaction, steering, max-turns, empty-response handling,
retry) while preserving Gaggle's checkpointing between operations.

## Decision

Adopt an operation-pipeline shape for the agent turn loop, mirroring Goose's
state machine, but keep Gaggle's durable-run and checkpoint model as the
source of truth. Each operation emits a `run_event` and may checkpoint before
the next operation. Steering messages are folded into the next inference input.

## Consequences

- Improved testability and observability of individual operations.
- Requires careful transaction boundaries so checkpoints remain consistent.
- Backwards-compatible with existing runs; recovery resumes from the latest
  checkpoint.
