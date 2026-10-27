---
name: provider-integrations
description: Integrate AI model providers with normalized streaming, tool calls, authentication, errors, and usage reporting.
version: 1.0.0
---

# Provider Integration Skill

## Mission
Support multiple model providers behind a consistent, testable interface without pretending their capabilities or authentication flows are identical.

## Adapter responsibilities
- Provider and model metadata.
- Authentication and credential lookup.
- Request serialization and response normalization.
- Streaming events and tool-call parsing.
- Capability discovery or explicit configuration.
- Timeouts, cancellation, rate limits, and error mapping.
- Usage and cost metadata when actually supplied.
- Safe diagnostics without leaking credentials or private prompts.

## Rules
- Keep provider-specific behavior out of the agent loop.
- Validate URLs and reject unsupported schemes.
- Do not assume every model supports tools, vision, reasoning, or the same context length.
- Do not silently switch models or send data to a different provider.
- Retry only safe requests, with bounded backoff.
- Handle malformed and partial streaming responses.
- Treat provider output as untrusted input.
- Store credentials in the OS credential store and reference them by identifier.
- Avoid logging raw request bodies or authorization headers.
- Use mock servers and fixtures for deterministic tests.

## Acceptance criteria
Every adapter has tests for success, streaming, tool calls, malformed response, authentication failure, rate limit, timeout, cancellation, and usage metadata where supported.
