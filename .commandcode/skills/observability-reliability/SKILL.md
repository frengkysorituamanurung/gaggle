---
name: observability-reliability
description: Make Gaggle diagnosable and resilient through structured logs, metrics, tracing, recovery, and actionable errors.
version: 1.0.0
---

# Observability and Reliability Skill

## Mission
Make failures explainable without leaking private user data.

## Required signals
Where appropriate, record run duration, provider latency, time to first token, tool duration, tool failure categories, cancellation, recovery outcome, queue depth, and resource-limit events.

## Logging
- Use structured tracing with correlation IDs for chat, run, and tool call.
- Redact secrets and sensitive arguments before serialization.
- Avoid full prompts, source files, environment dumps, and raw command output in default logs.
- Keep error messages actionable but non-sensitive.
- Make diagnostic export explicit and allow review before sharing.

## Reliability
- Bound retries and use backoff only for safe operations.
- Define deadlines and cancellation behavior.
- Make run state transitions observable.
- Handle partial success honestly.
- Detect stale heartbeats and uncertain outcomes.
- Avoid unbounded queues, buffers, and task spawning.
- Provide useful recovery instructions instead of silently discarding state.

## Acceptance criteria
A user can identify which run/tool failed, understand the failure category, and decide what to do next without exposing secrets or reading raw internal logs.
