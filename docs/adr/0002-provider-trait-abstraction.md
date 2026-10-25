# ADR 0002 — Provider Trait Abstraction

- Status: Proposed
- Date: 2026-10-25

## Context

Gaggle normalizes all model providers to `ModelCompletionResponse` and
`ModelStreamEvent` through `ModelProviderAdapter`. Phase B adds Anthropic,
Ollama, and ChatGPT adapters. Provider-specific auth and streaming formats
must not leak into the agent runtime.

## Decision

Keep a single `ModelProviderAdapter` trait with structured `ProviderError`,
streaming normalization, and retry/backoff handled inside adapters. Store only
`secret_ref` identifiers for credentials; never persist plaintext tokens. New
adapters (Anthropic, Ollama, ChatGPT OAuth) implement the trait without changes
to `AgentRuntime`.

## Consequences

- Agent runtime stays provider-agnostic.
- Adapter implementations own protocol details and error mapping.
- Testing uses mock servers per adapter.
