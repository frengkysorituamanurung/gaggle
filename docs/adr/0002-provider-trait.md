# ADR 0002 — Provider Trait

- Status: Proposed
- Date: 2026-10-25

## Context

Gaggle normalizes all model providers to `ModelCompletionResponse` and
`ModelStreamEvent` through `ModelProviderAdapter`. Provider-specific auth and
streaming formats must not leak into the agent runtime. New adapters
(Anthropic, Ollama, ChatGPT OAuth, and later providers) must be added without
changing `AgentRuntime`.

## Decision

Keep a single `ModelProviderAdapter` trait with structured `ProviderError`,
streaming normalization, and retry/backoff handled inside adapters. Store only
`secret_ref` identifiers for credentials; never persist plaintext tokens. New
adapters implement the trait without changes to `AgentRuntime`.

## Consequences

- Agent runtime stays provider-agnostic.
- Adapter implementations own protocol details and error mapping.
- Testing uses mock servers per adapter.
- No plaintext tokens in SQLite or logs.
