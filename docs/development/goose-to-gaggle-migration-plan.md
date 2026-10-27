# Goose → Gaggle Migration Plan

> Note: The UI section and Phase G are superseded by
> `docs/development/ui-typescript-rewrite-plan.md`.

This plan translates capabilities from the vendored Block Goose reference
(`goose/`) into Gaggle's local-first, project-centric architecture. It is a
**migration roadmap**, not a single implementation task. Each phase is sized to
be delivered independently and must pass Gaggle's quality gates before the next
phase starts.

## 0. Scope and principles

- **Source of truth**: `goose/` (Block Goose, Rust + Electron). Only read from
  it; do not modify it.
- **Target**: `crates/*` and `docs/*` of the Gaggle workspace.
- **Direction**: port *behaviour and contracts*, not file layout. Gaggle keeps
  its crate boundaries (`gaggle-core`, `gaggle-tools`, `gaggle-mcp`, …) and its
  SQLite schema as the system of record.
- **Non-negotiables** (from `AGENTS.md`): server-side authorization, default
  `ask` permission mode, canonicalized project roots, no plaintext secrets,
  short transactions, English-only code/docs, and the `fmt`/`clippy`/`test`
  quality gates.
- **Out of scope for this plan**: Windows support, multi-user/team features,
  native model inference, and Goose's Electron desktop shell (Gaggle stays a
  single embedded HTML document served by `gaggle-server`).

## 1. Source → target map

| Goose area | Key source paths | Gaggle target | Current status |
| :--- | :--- | :--- | :--- |
| Agent state machine | `goose/crates/goose-agent/src/machine.rs`, `goose/crates/goose/src/agents/state_machine/*` | `crates/gaggle-core/src/agent_loop.rs`, `run_engine.rs` | Partial: bounded loop, streaming, approvals, cancellation, checkpoints exist. Missing state-machine op pipeline (steering, skills, slash commands, stop hooks, project context). |
| MCP client | `goose/crates/goose/src/agents/mcp_client.rs`, `goose/crates/goose-mcp/*` | `crates/gaggle-mcp/src/lib.rs` | Strong: stdio/http/sse transports, discovery, registry. Missing connection reuse/caching and MCP resources/prompts (intentionally out of scope). |
| Providers | `goose/crates/goose-providers/src/*`, `goose/crates/goose/src/providers/*` | `crates/gaggle-core/src/provider.rs`, `provider_oauth.rs` | Partial: OpenAI-compatible + Mock + (per server) Anthropic/ChatGPT/Ollama adapters. Missing Google, Azure, OpenRouter, Databricks, Snowflake, local-inference. |
| Context management | `goose/crates/goose-context-management/src/*`, `goose/crates/goose/src/context_mgmt/mod.rs` | `crates/gaggle-core/src/compaction.rs`, `context_assembler.rs` | Partial: basic summary compaction. Missing structured summaries, tool-pair compaction, continuation text, token-limit heuristics. |
| Core agent | `goose/crates/goose/src/agents/agent.rs`, `session/*`, `permission/*` | `crates/gaggle-core/*`, `gaggle-db` | Partial: sessions/chats, permission engine, runs. Missing steering, hooks, MOIM, slash commands, final-output tool. |
| UI | `goose/ui/desktop/src/*` | `crates/gaggle-ui`, `gaggle-server` (embedded HTML) | Partial: 3-panel shell, chat, approvals. Missing settings/diagnostics/first-run/onboarding, memory & recipe management UI. |
| Recipes / scheduler | `goose/crates/goose/src/recipe/*`, `scheduler/*` | `crates/gaggle-recipes`, `gaggle-db` (recipes/schedules) | Partial/early: crate exists; verify parity with Goose recipe params, sub-recipes, cron. |
| Memory | `goose/crates/goose-mcp/src/memory/*` | `gaggle-db` (memories), `gaggle-tools` | Partial: tables + FTS5 exist; runtime CRUD/search partly implemented. |
| Browser / PTY | `goose/crates/...` (computer controller) | `gaggle-tools` (browser_tools, terminal) | Partial: planned in roadmap M11/M12. |

## 2. Parity snapshot (already done in Gaggle)

- Bounded agent loop with `DEFAULT_MAX_AGENT_STEPS`, streaming SSE, tool-call permission routing, checkpoints, run recovery, cancellation registry.
- 7-step `PermissionEngine`, `command_rules`, approvals, `verify_approved_execution`.
- MCP client transports (stdio / streamable HTTP / SSE), SSRF validation, extension registry, tool discovery, active-tool limit guard.
- Provider abstraction with streaming normalization and structured `ProviderError`.
- Context assembly with `AGENTS.md` discovery, project instructions, memories, tool definitions.
- Basic auto/manual compaction persisted to `chat_summaries`.
- SQLite schema with 37 ERD models, FTS5, WAL, migrations.
- Worktree/PTY/browser/recipes/memory tables present.

## 3. Migration phases

### Phase A — Agent runtime parity (highest value)

Goal: bring Gaggle's agent loop to Goose's state-machine robustness without
abandoning Gaggle's durable-run/checkpoint model.

Tasks:
1. **Operation pipeline**: refactor `AgentRuntime::run_turn_inner` into discrete
   operations (LLM inference, tool calling, approval, compaction, steering,
   max-turns, empty-response handling, retry) mirroring
   `goose/crates/goose/src/agents/state_machine/*`, while keeping Gaggle's
   `RunRepository` checkpointing between operations.
2. **Steering support**: accept mid-run user messages (`steer`) and fold them
   into the next inference input; persist as `run_events`.
3. **Tool lifecycle parity**: tool-pair compaction, large-response handling,
   tool-call labels, and consistent error envelopes (compare
   `goose/crates/goose/src/agents/tool_execution.rs`,
   `large_response_handler.rs`).
4. **Subagent execution**: implement child-chat spawning with
   `parent_chat_id` / `parent_tool_call_id`, inherited permissions, and
   sequential-when-repeated guard (roadmap M18).
5. **Approval UX parity**: richer approval kinds (command, file diff, network)
   and verification before execution (already partially in M3).

Exit criteria:
- All new operations covered by unit + integration tests.
- `cargo test --workspace` green; no clippy warnings.

### Phase B — Provider parity

Goal: support Anthropic native, Ollama, ChatGPT OAuth, and (later) Google/Azure.

Tasks:
1. Add `AnthropicAdapter` (x-api-key, anthropic-version, SSE streaming) reusing
   `ANTHROPIC_API_VERSION` constants already in `provider.rs`.
2. Add `OllamaAdapter` (local endpoint, no auth, NDJSON streaming).
3. Add `ChatGptAdapter` OAuth (reuse `gaggle-core::provider_oauth` and
   credential store; store only `secret_ref`).
4. Normalize all adapters to `ModelCompletionResponse` + `ModelStreamEvent`.
5. Provider-specific error mapping and retry/backoff (compare
   `goose/crates/goose-providers/src/http_status.rs`, `retry`).

Exit criteria:
- Mock-server integration tests per adapter.
- No plaintext tokens in SQLite/logs.

### Phase C — Context management parity

Goal: smarter compaction that preserves security-relevant context and tool pairs.

Tasks:
1. Port structured summary template from `goose-context-management` (`structured.rs`,
   `summarize.rs`, `templates.rs`) into `gaggle-core::compaction`.
2. Implement tool-pair summarization (batch tool request/response into one
   summary) with continuation text.
3. Token-limit heuristics: `auto_compact_threshold` combining ratio + absolute
   token cap (compare Goose `context_mgmt/mod.rs`).
4. Preserve unresolved approvals, system prompts, and critical constraints.
5. Manual compaction endpoint already exists; add preview/dry-run option.

Exit criteria:
- Compaction tests verify no dropped approvals.
- FTS/memory context still bounded by budget.

### Phase D — MCP client hardening

Goal: make MCP connections production-grade.

Tasks:
1. Add connection pooling/reuse per extension instead of connect-per-call in
   `McpToolExecutor`.
2. Add MCP `notifications`, progress, and cancellation propagation.
3. Capability gating per `extension_permissions` (already present) with audit
   logs.
4. Add resources/prompts only if needed (currently non-goal; revisit after D).

Exit criteria:
- Stress test with ≥ 3 concurrent extensions under tool limit 25.

### Phase E — Memory, recipes, scheduler

Goal: finish M10/M14 runtime.

Tasks:
1. Memory CRUD + FTS5 search + pinned prioritization + soft delete + optional
   embeddings.
2. Recipe parser (inline/YAML/URL), `minijinja` rendering, param validation,
   sub-recipes with recursion guard.
3. Cron scheduler loop (60s) with missed-run handling and timezone support
   (`croner`).
4. Memory export to `.pal/memory/*.md` (optional).

Exit criteria:
- Integration tests with temp SQLite and mock cron.

### Phase F — Worktree, PTY, browser

Goal: finish M8/M11/M12.

Tasks:
1. `WorktreeManager` with git worktree add/remove/merge/discard + audit logs.
2. PTY sessions via `portable-pty` + WebSocket streaming + process-tree cancel.
3. Browser sandbox via `chromiumoxide` with isolated profiles + SSRF guards.
4. Tool routing through `PermissionEngine` for all three.

Exit criteria:
- Cross-platform tests on macOS + Linux.
- Security tests for path traversal/symlink escape.

### Phase G — UI parity

Goal: expose backend capabilities in the embedded UI.

Tasks:
1. Settings screen (providers, models, credentials status, permissions, theme).
2. First-run onboarding (provider setup + project registration).
3. Diagnostics view (`gaggle doctor` output).
4. Memory/recipe/schedule management panels.
5. Worktree diff/merge/discard UI.

Exit criteria:
- UI/backend integration audit updated (`docs/development/ui-backend-integration-audit.md`).

### Phase H — Observability, security, docs

Tasks:
1. Structured tracing redaction review (compare Goose telemetry).
2. Audit logs for all new lifecycle events.
3. Update `known-limitations.md` as capabilities land.
4. ADRs for major decisions (state machine refactor, provider trait changes).

### Phase I — Quality and release

Tasks:
1. Keep `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
   `cargo test --workspace` green at each phase.
2. Add migration smoke tests and update `goose-self-test`-style self-test recipe if
   applicable.
3. Version bump and release notes.

## 4. Dependency order

```
A (agent) ──► C (context) ──► B (providers, partially parallel)
D (mcp)  ──► E (memory/recipes) ──► F (worktree/pty/browser)
G (ui) depends on E,F
H (obs) continuous
I (release) last
```

Phase A and D can run in parallel; B and C are coupled (compaction needs a
provider). E depends on D (extensions) and B (provider for memory embeddings).
F is independent of E but shares `PermissionEngine`. G depends on E and F.

## 5. Risk register

| Risk | Impact | Mitigation |
| :--- | :--- | :--- |
| State-machine refactor breaks durable recovery | High | Keep checkpointing between operations; add recovery tests before refactor. |
| Provider OAuth token leakage | High | Store only `secret_ref`; redact logs; OS keychain wiring (known limitation). |
| MCP connection reuse introduces stale sessions | Medium | Add health checks + reconnect; keep connect-per-call fallback. |
| Compaction drops security context | High | Preserve approvals/system prompts; tests assert invariants. |
| Scope creep across 9 phases | High | One phase per PR; update roadmap milestones as phases land. |
| UI as single HTML doc limits complexity | Medium | Keep panels modular; consider code-splitting only if needed. |

## 6. Verification gates (every phase)

- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- New integration tests for the phase's exit criteria.
- Update `docs/development/known-limitations.md` when a limitation is removed.

## 7. Recommended first PR

Start with **Phase C (context management parity)** combined with a small slice
of Phase A (operation extraction for compaction only). It is self-contained,
has clear tests, and unblocks longer conversations immediately. Follow with
Phase B (Anthropic + Ollama) to diversify providers.
