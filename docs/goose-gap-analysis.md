# Gaggle vs Goose — Gap Analysis

This document compares **Gaggle** (`frengky/gaggle`) with **Goose** (`aaif-goose/goose`, vendored under `goose/`) and records which capabilities are missing in Gaggle and their implementation status.

## Already present in Gaggle (parity or ahead)
- Project-centric workspace, durable runs, run events SSE replay, checkpoints.
- Git worktree isolation per chat, changes panel with review comments.
- 7-step permission engine (chat_only, command_rules deny, full, tool_permissions, read-only, smart risk cache, default approval).
- Unified extensions + MCP (stdio/streamable_http/sse/wasm), active tool limit 25.
- Recipes, sub-recipes, subagents, cron scheduler.
- FTS5 search (messages/memories) + optional embeddings.
- Multi-provider streaming (OpenAI-compat, Anthropic, Ollama, ChatGPT OAuth PKCE).
- Bounded context assembly with secret redaction.
- Secret references only (no plaintext keys in DB).
- SQLite conventions (UUIDv7, ms timestamps, CHECK constraints, WAL/FK).
- Desktop packaging for macOS (.app) and Linux (.desktop + install.sh).
- CI/CD: quality gates on ubuntu+macos, release pipeline for Mac+Linux, smoke tests.

## Identified gaps (Goose has, Gaggle lacks or partial)
1. **Native desktop window.** Gaggle now has a Tauri shell that embeds the Axum UI on a dynamically allocated loopback port. Goose ships an Electron app with a richer ACP-based desktop workflow; UI and lifecycle parity remain incomplete.
2. **One-line installer.** Goose provides `download_cli.sh`. Gaggle now ships `scripts/download_cli.sh` (Mac/Linux) for installing from GitHub releases.
3. **ACP (Agent Client Protocol).** Goose has `goose-acp` / `goose-acp-macros` and an ACP client. Gaggle has no ACP transport yet.
4. **Local inference.** Goose has `goose-local-inference` (llama.cpp) with optional GPU backends. Gaggle has no in-process local model inference; only Ollama as an external provider.
5. **Download manager.** Goose has `goose-download-manager` (model/asset downloads with progress). Gaggle lacks a centralized download manager.
6. **Internationalization (i18n).** Goose UI supports many locales. Gaggle UI has no i18n framework.
7. **Goosehints / project hints.** Goose reads `.goosehints`. Gaggle relies on `AGENTS.md` and project instructions; equivalent but different convention.
8. **OIDC proxy / enterprise auth.** Goose ships `oidc-proxy`. Gaggle has no OIDC proxy.
9. **Buzz automation / advanced evals.** Goose has `evals/` and buzz automation. Gaggle has tests but no eval harness.
10. **Windows support.** Goose supports Windows. Gaggle packaging targets only macOS and Linux (by design).

## Implementation status in this change set
- Added `scripts/download_cli.sh` (gap #2).
- Improved desktop launcher to open a dedicated app window (gap #1, partial).
- Added graceful shutdown to the server (supports installed-app lifecycle).
- Added read-only import of one Goose SQLite session into a selected Gaggle project; usage and opaque metadata are preserved in redacted import-artifact tables, while credentials are not copied.
- Added provider-native model catalog discovery for OpenAI-compatible, Google, Anthropic, and Ollama endpoints.
- Produced this gap analysis (documentation).

## Recommended next steps (via CI, no local builds)
- ACP transport crate (`gaggle-acp`) mirroring Goose's ACP, integrated with run events.
- Optional local inference crate wrapping llama.cpp (feature-gated GPU backends).
- i18n extraction for `gaggle-ui`.
- Windows packaging target in release workflow.
- Eval harness under `evals/`.
