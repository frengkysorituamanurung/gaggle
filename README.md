# Gaggle

[![License: Apache 2.0](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](./LICENSE)
[![Rust Edition](https://img.shields.io/badge/Rust-2024_Edition-orange.svg)](https://www.rust-lang.org/)
[![Database: SQLite](https://img.shields.io/badge/Database-SQLite_WAL_%2B_FTS5-003B57.svg)](https://www.sqlite.org/)

**Gaggle** is a self-hosted personal AI agent designed to work directly inside your real local projects and workspaces. Written entirely in **Rust**, Gaggle uses a single global **SQLite** database per device.

Gaggle's architecture is inspired by **Goose** (Block's open-source Rust AI agent), while introducing key capabilities tailored for local software engineering: **Project as a first-class entity**, **per-chat Git Worktree isolation**, a **Changes panel with line-level review comments**, **Durable Runs** that survive browser tab closures and server restarts, a **Code Mode sandbox**, **PTY Terminal sessions**, and a **Sandboxed Browser** agent.

---

## Key Features

- **Project-Centric Workspace**: Every conversation (`chats`), memory entry (`memories`), shell rule (`command_rules`), Git worktree (`worktrees`), cron job (`schedules`), and terminal session (`terminal_sessions`) is scoped cleanly under a `projects` entity.
- **Durable Runs & Event Replay**: Agent executions (`runs`) persist independently of connected clients. Every event is recorded sequentially in `run_events` (the single source of truth for SSE replay), and `run_checkpoints` allow unfinished runs to resume automatically upon server restart.
- **Git Worktree Mode & Changes Panel**: Isolate each chat's file modifications in a dedicated Git worktree (`worktrees`), inspect unified diffs per run in `file_changes`, and leave inline review comments (`review_comments`) that can be dispatched back to the agent.
- **Layered 7-Step Permission Engine**:
  - Four permission modes per chat: `ask` (safe default), `smart` (LLM risk evaluation cache), `full` (autonomous), and `chat_only`.
  - Shell command rules (`command_rules` with `allow`, `ask`, and `deny` actions — where `deny` is strictly enforced even in `full` mode).
  - Per-tool permissions (`always_allow`, `ask_before`, `never_allow`) configurable globally or overridden per project.
- **Unified Extension & MCP Architecture**: Built-in capabilities (`gaggle.fs`, `gaggle.shell`, `gaggle.memory`, `gaggle.web`, `gaggle.browser`) and external Model Context Protocol (MCP) servers (`stdio`, `streamable_http`, `sse`, `wasm`) share a unified `extensions` and `extension_tools` model.
- **Recipes, Sub-Recipes, & Cron Scheduler**: Package instructions, parameterized prompts (`minijinja`), and required extensions into reusable `recipes`. Delegate tasks to isolated subagent chats (`parent_chat_id`) or run recipes on a recurring cron schedule (`schedules`).
- **Full-Text & Semantic Search**: Built-in SQLite FTS5 virtual tables for instant search across chat history (`messages_fts`) and project/global memories (`memories_fts`), alongside optional vector embeddings (`memory_embeddings`).
- **Keychain-Backed Secrets**: API keys and OAuth tokens are **never** stored in SQLite. The database stores only a `secret_ref` identifier pointing to the host OS Keychain.

---

## Workspace Architecture

Gaggle is organized as a multi-crate Cargo workspace with clear domain boundaries:

```text
gaggle/
├─ Cargo.toml                # Cargo workspace manifest
├─ LICENSE                   # Apache-2.0 License
├─ README.md                 # Project overview and usage guide
├─ CONTRIBUTING.md           # Contribution guidelines
├─ CODE_OF_CONDUCT.md        # Community Code of Conduct
├─ SECURITY.md               # Security policy and vulnerability reporting
├─ docs/
│  └─ ERD.md                 # Database schema and ERD documentation
└─ crates/
   ├─ gaggle-core/           # Agent loop, durable run recovery, compaction, 7-step permission engine, Code Mode
   ├─ gaggle-db/             # SQLite connection (WAL, FK), sqlx migrations, 37 ERD models, domain repositories
   ├─ gaggle-tools/          # Built-in extensions (fs, shell, memory, web, browser) and unified diff generation
   ├─ gaggle-mcp/            # MCP client integration, extension registry, active tool limit guard (~25 tools)
   ├─ gaggle-recipes/        # Recipe parser, required/optional parameter validation, minijinja templates, scheduler
   ├─ gaggle-server/         # Axum HTTP/SSE server, REST API, chat JSON export/import, embedded Web UI serving
   ├─ gaggle-cli/            # `gaggle` CLI binary (`serve`, `project add/list`, `db-path`)
   └─ gaggle-ui/             # Web UI shell served by gaggle-server
```

---

## Storage Design & Database Conventions

Gaggle uses a **single global SQLite database** across all projects. Storing the database globally prevents polluting project repositories with `.gaggle/` folders, supports read-only workspaces, avoids duplicating databases across Git worktrees, and preserves chat history if a project directory is moved or deleted.

Default database locations (resolved via the `directories` crate):
- **macOS**: `~/Library/Application Support/gaggle/gaggle.db`
- **Linux**: `~/.local/share/gaggle/gaggle.db`
- **Windows**: `%APPDATA%\gaggle\data\gaggle.db`

Core SQLite conventions enforced in `gaggle-db`:
- **Primary Keys**: Time-ordered UUIDv7 stored as `TEXT`.
- **Timestamps**: Unix epoch milliseconds stored as `INTEGER`.
- **JSON & Enum Integrity**: Enforced at the schema level via `CHECK(json_valid(...))` and `CHECK(column IN (...))`.
- **Connection PRAGMAs**: `journal_mode=WAL`, `foreign_keys=ON`, `synchronous=NORMAL`, `busy_timeout=5000`, and `auto_vacuum=INCREMENTAL`.

See [`docs/ERD.md`](./docs/ERD.md) for the complete Entity-Relationship Diagram and table specifications.

---

## Getting Started

### Prerequisites

- **Rust Toolchain**: Latest stable Rust toolchain supporting the 2024 Edition (`rustc >= 1.85`).
- **Git**: Required for repository management and Git Worktree isolation.

### Build & Test

1. Clone the repository:
   ```bash
   git clone https://github.com/frengky/gaggle.git
   cd gaggle
   ```

2. Build all workspace crates:
   ```bash
   cargo build --workspace
   ```

3. Run the test suite (including SQLite migrations, FTS5 triggers, and Permission Engine tests):
   ```bash
   cargo test --workspace
   ```

---

## CLI Usage (`gaggle`)

The `gaggle` binary is provided by the `gaggle-cli` crate.

### 1. Print the Global Database Path
```bash
cargo run -p gaggle-cli -- db-path
```

### 2. Run Diagnostics & Database Health Check
```bash
cargo run -p gaggle-cli -- doctor
```

### 3. Register a Project Workspace
```bash
cargo run -p gaggle-cli -- project add "Gaggle Workspace" "/path/to/your/project" --vcs git
```

### 4. List Active Projects
```bash
cargo run -p gaggle-cli -- project list
```

### 5. Start the HTTP/SSE Server & Web UI
```bash
cargo run -p gaggle-cli -- serve --addr 127.0.0.1:4321
```
On startup, Gaggle automatically:
1. Opens the global SQLite pool in WAL mode and applies any pending `sqlx` migrations.
2. Registers all bundled built-in extensions (`gaggle.fs`, `gaggle.shell`, `gaggle.memory`, `gaggle.web`, `gaggle.browser`).
3. Safely recovers any durable runs left in `running` or `waiting_approval` state by expiring stale approvals and transitioning interrupted runs to `paused`.

### Core HTTP Endpoints
- `GET /` — Serves the Gaggle Web UI.
- `GET /api/health` — Health check endpoint (`{"status": "ok", "service": "gaggle"}`).
- `GET /api/diagnostics` — Returns a redacted diagnostic report of OS, SQLite PRAGMAs, migrations, and active counts.
- `GET /api/projects` — Lists all active projects.
- `POST /api/projects` — Validates, canonicalizes, and registers a new project workspace.
- `GET /api/projects/{id}/files` — Lists validated files and directories inside the project workspace.
- `GET /api/projects/{id}/git-status` — Inspects the project's Git branch and modified/untracked files.
- `GET /api/projects/{id}/diff` — Generates a bounded unified diff of uncommitted workspace changes.
- `POST /api/chats` — Creates a new chat session within a project.
- `GET /api/chats/{id}/export` — Exports a complete chat session (`chat`, `messages`, and `message_parts`) as a portable JSON bundle.
- `GET /api/approvals/pending` — Lists all pending tool-call approval requests.
- `POST /api/approvals/{id}/decide` — Records an approval/denial decision, optionally remembers a `command_rules` entry, and appends an `audit_logs` record.

---

## Contributing & Community

Contributions are welcome! Please review the following documents before opening an issue or submitting a pull request:

- [Contributing Guide (CONTRIBUTING.md)](./CONTRIBUTING.md)
- [Code of Conduct (CODE_OF_CONDUCT.md)](./CODE_OF_CONDUCT.md)
- [Security Policy (SECURITY.md)](./SECURITY.md)

---

## License

Licensed under the **Apache License, Version 2.0**. See the [`LICENSE`](./LICENSE) file for full details.
