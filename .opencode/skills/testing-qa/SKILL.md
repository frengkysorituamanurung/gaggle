---
name: testing-qa
description: Define and run Gaggle's unit, integration, end-to-end, security, recovery, and cross-platform tests.
version: 1.0.0
---

# Testing and QA Skill

## Mission
Provide evidence that Gaggle works as specified and fails safely.

## Test strategy
- Unit tests: domain rules, permissions, state transitions, parsing, and normalization.
- Integration tests: SQLite migrations, provider adapters, filesystem tools, process execution, and event replay.
- End-to-end tests: onboarding, prompt submission, approval, cancellation, diff review, and recovery.
- Security tests: traversal, symlink escape, injection, secret redaction, approval replay, SSRF, and malicious extension output.
- Fault tests: provider timeout, database busy, disk full, process crash, application restart, and partial tool failure.
- Cross-platform tests: supported macOS/Linux OS and architecture matrix.

## Rules
- Use deterministic fixtures and mocked providers where possible.
- Avoid paid API calls in normal CI.
- Test both success and failure paths.
- Do not use sleeps as the primary synchronization method when events or conditions can be awaited.
- Never claim a test passed unless it ran and passed.
- Keep regression tests for fixed defects.
- Do not weaken or delete a failing test merely to make CI green without understanding the underlying issue.

## Reporting
For each validation run, list the exact command, environment/target, pass/fail result, and relevant failure details. Separate tests not run from tests that failed.
