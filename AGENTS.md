# Gaggle Project Instructions

These instructions are automatically loaded by OpenCode in every session within the **Gaggle** repository.

## Mandatory Skill Loading Router (Execute Before Any Action)

Before planning, reading, writing, or modifying any code in response to a user prompt, you **MUST** match the user's request against the routing table below and invoke the `skill` tool with the corresponding skill `id` first:

| User Prompt Topic / Crate Target | Required Skill ID to Call via `skill` Tool |
| :--- | :--- |
| SQLite tables, SQL migrations, `sqlx` queries, indexes, FTS5 (`memories_fts`, `messages_fts`), model structs, or repositories in `crates/gaggle-db` | `gaggle-db-schema` |
| Agent loop, durable runs (`runs`, `run_events`, `run_checkpoints`), 7-step `PermissionEngine`, `command_rules`, `approvals`, Code Mode, compaction (`crates/gaggle-core`), built-in tools (`crates/gaggle-tools`), MCP registry (`crates/gaggle-mcp`), recipes & cron schedules (`crates/gaggle-recipes`), or HTTP/SSE endpoints (`crates/gaggle-server`, `crates/gaggle-cli`, `crates/gaggle-ui`) | `gaggle-architecture` |
| Tasks that span both database repositories and core/server/tool logic | Load **both** `gaggle-db-schema` and `gaggle-architecture` in parallel |
| Security reviews, vulnerability checks, keychain `secret_ref`, sandboxing, or permission bypass audits | `security-audit` (and `gaggle-architecture`) |

**Rule**: Never skip calling the `skill` tool when a prompt touches any of the areas above, unless that skill is already loaded in the current session context.

---

## Workspace Architecture (`gaggle`)
- `crates/gaggle-db`: Global SQLite connection (`gaggle.db`), PRAGMA configuration (`WAL`, `foreign_keys=ON`, `synchronous=NORMAL`, `busy_timeout=5000`, `auto_vacuum=INCREMENTAL`), `sqlx` migrations, 37 ERD table models, FTS5 virtual tables (`memories_fts`, `messages_fts`), and domain repositories.
- `crates/gaggle-core`: Core agent loop logic, durable run recovery (`RunEngine`), 7-step `PermissionEngine`, context compaction, and Code Mode sandbox configuration.
- `crates/gaggle-tools`: Built-in extensions (`gaggle.fs`, `gaggle.shell`, `gaggle.memory`, `gaggle.web`, `gaggle.browser`) and unified diff generation (`similar`).
- `crates/gaggle-mcp`: MCP client integration and extension registry (enforcing the recommended `<= 25` active tools limit per chat).
- `crates/gaggle-recipes`: Recipe parser, required vs. optional parameter validation, `minijinja` template rendering, and `croner` scheduler.
- `crates/gaggle-server`: Axum HTTP/SSE server, REST API endpoints, chat JSON export/import, and Web UI serving.
- `crates/gaggle-cli`: The `gaggle` command-line binary.
- `crates/gaggle-ui`: Web UI shell for Gaggle.

## Core Invariants
1. **Database Conventions**: UUIDv7 as `TEXT`, Unix epoch milliseconds as `INTEGER`, `CHECK(json_valid(...))`, `CHECK(col IN (...))`, and `secret_ref` for OS Keychain secrets (never store plaintext credentials in SQLite).
2. **Permission Decision Order**: Strictly preserve `chat_only` -> `command_rules` deny -> `full` -> `tool_permissions` -> `is_read_only` -> `smart` `tool_risk_cache` -> `approvals`.
3. **Language Policy**: All code, comments, commit messages, documentation, and skills in this repository must be written in English.
4. **Verification**: Always run `cargo test --workspace` after modifying any Rust or SQL files.
