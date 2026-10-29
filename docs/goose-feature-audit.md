# Goose vs Gaggle — Feature Audit

Date: 2026-10-29
Goose snapshot: `775b3958c`

## Summary

| Category | Goose | Gaggle | Status |
|---|---|---|---|
| Agent runtime | State machine with operations | Bounded loop + extracted operations | Partial |
| Providers | 15+ (OpenAI, Anthropic, Google, Ollama, Azure, Databricks, Snowflake, OpenRouter, etc.) | 11 (OpenAI-compat, Anthropic, Ollama, Google Gemini, Azure, Databricks, Snowflake, + OpenAI-compat providers) | Partial |
| MCP client | Full MCP with resources/prompts | MCP with tools only | Partial |
| Context management | Structured summaries, tool-pair compaction | Basic compaction | Partial |
| Memory | CRUD, FTS5, embeddings, export | CRUD, FTS5, embeddings, export | Parity |
| Recipes | YAML/inline/URL, params, sub-recipes, cron | YAML/inline/URL, params, sub-recipes, cron | Parity |
| Subagents | Child chats with inherited permissions | Child chats with inherited permissions | Parity |
| Worktree | Git worktree add/remove/merge/discard | Git worktree add/remove/merge/discard | Parity |
| PTY terminal | portable-pty + WebSocket | portable-pty + WebSocket | Parity |
| Browser sandbox | chromiumoxide | chromiumoxide | Parity |
| Permission engine | 7-step decision | 7-step decision | Parity |
| Extensions | MCP + builtins + platform extensions | MCP + builtins | Partial |
| Desktop | Electron + ACP | Tauri + Axum (Electron rebranded) | Partial |
| CLI | Full CLI with ACP | Full CLI | Partial |
| ACP | Full ACP server | Minimal endpoints (/health, /status) | Partial |
| i18n | 15 locales | 16 locales | Parity |
| Local inference | llama.cpp with GPU | None (Ollama only) | Missing |
| OIDC proxy | Yes | No | Missing |
| ACP client | Yes | No | Missing |
| Download manager | Yes | No | Missing |
| Windows support | Yes | No (by design) | N/A |
| Eval harness | Yes (buzz) | Basic tests | Partial |
| Slash commands | Yes | No | Missing |
| Hooks | Yes | Partial (pre-tool-use) | Partial |
| MOIM (Model-Owned Instruction Memory) | Yes | No | Missing |
| Final output tool | Yes | No | Missing |
| Steering | Yes | Yes | Parity |
| Slash commands | Yes | No | Missing |
| Code mode | Yes (boa_engine) | Yes (boa_engine) | Parity |
| Snapshots | Yes | No | Missing |
| GenAI telemetry | Yes | Basic tracing | Partial |
| Computer controller | Yes (peekaboo) | No | Missing |
| Adversary mode | Yes | No | Missing |
| Live voice | Yes | No | Missing |
| Scheduler | Yes (cron) | Yes (cron) | Parity |
| Session export/import | Yes | Yes (chat bundle) | Parity |
| Data backup/restore | Yes | Yes | Parity |

## Detailed Gap Analysis

### Missing in Gaggle
1. **Local inference** — Goose has `goose-local-inference` (llama.cpp). Gaggle only supports Ollama externally.
2. **OIDC proxy** — Goose ships `oidc-proxy` for enterprise auth.
3. **ACP client** — Goose has ACP client for connecting to external agents.
4. **Download manager** — Goose has centralized model/asset download with progress.
5. **Slash commands** — Goose supports slash commands in the agent loop.
6. **Hooks (full)** — Goose has pre/post tool use hooks, stop hooks, etc.
7. **MOIM** — Goose has Model-Owned Instruction Memory.
8. **Final output tool** — Goose has a dedicated final output tool.
9. **Snapshots** — Goose has session snapshots for undo/redo.
10. **GenAI telemetry** — Goose has OpenTelemetry integration.
11. **Computer controller** — Goose has peekaboo for macOS automation.
12. **Adversary mode** — Goose has adversarial testing mode.
13. **Live voice** — Goose has voice interaction.
14. **Steering (full)** — Goose has more advanced steering.

### Partial in Gaggle
1. **Agent runtime** — Gaggle has bounded loop with extracted operations; Goose has full state machine.
2. **Providers** — Gaggle has 11 providers; Goose has 15+.
3. **MCP client** — Gaggle has tools only; Goose has resources and prompts too.
4. **Context management** — Gaggle has basic compaction; Goose has structured summaries.
5. **Extensions** — Gaggle has MCP + builtins; Goose has platform extensions too.
6. **Desktop** — Gaggle has Tauri + Electron (rebranded); Goose has full Electron.
7. **CLI** — Gaggle has basic CLI; Goose has full CLI with ACP.
8. **ACP** — Gaggle has minimal endpoints; Goose has full ACP server.
9. **Eval harness** — Gaggle has basic tests; Goose has buzz evals.

### Parity or ahead
1. **Memory** — Full parity (CRUD, FTS5, embeddings, export).
2. **Recipes** — Full parity (YAML/inline/URL, params, sub-recipes, cron).
3. **Subagents** — Full parity (child chats, inherited permissions).
4. **Worktree** — Full parity (add/remove/merge/discard).
5. **PTY terminal** — Full parity (portable-pty + WebSocket).
6. **Browser sandbox** — Full parity (chromiumoxide).
7. **Permission engine** — Full parity (7-step decision).
8. **i18n** — 16 locales (Goose has 15).
9. **Code mode** — Full parity (boa_engine).
10. **Scheduler** — Full parity (cron).
11. **Session export/import** — Full parity (chat bundle).
12. **Data backup/restore** — Full parity.
13. **Steering** — Full parity.