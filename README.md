# Gaggle

<p align="center">
  <img src="https://raw.githubusercontent.com/frengkysorituamanurung/gaggle/development/docs/assets/gaggle-logo.svg" alt="gaggle" width="184" height="50" />
</p>

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
- `GET /api/settings` — Lists persisted application settings (`app_settings`, including UI theme).
- `PUT /api/settings` — Upserts a JSON-validated application setting by key.
- `GET /api/projects` — Lists all active projects along with `path_exists: bool` workspace accessibility status.
- `POST /api/projects` — Validates, canonicalizes, and registers a new project workspace.
- `PATCH /api/projects/{id}` — Updates project defaults (`default_mode`, `default_access`, `default_model_id`).
- `GET /api/projects/{id}/chats` — Lists active conversations scoped to a project.
- `GET /api/projects/{id}/files` — Lists validated files and directories inside the project workspace.
- `GET /api/projects/{id}/git-status` — Inspects the project's Git branch and modified/untracked files.
- `GET /api/projects/{id}/diff` — Generates a bounded unified diff of uncommitted workspace changes.
- `GET /api/projects/{id}/worktrees` — Lists isolated Git worktrees associated with a project.
- `GET /api/projects/{id}/command-rules` — Lists global and project-scoped shell command rules.
- `POST /api/projects/{id}/command-rules` — Adds an `allow`, `ask`, or `deny` command rule (deny rules always win).
- `GET /api/projects/{id}/tool-permissions` — Lists enabled tools with their effective per-project permission level.
- `PUT /api/projects/{id}/tool-permissions` — Sets `always_allow`, `ask_before`, or `never_allow` for one tool.
- `POST /api/chats` — Creates a new chat session within a project (automatically provisioning an isolated Git worktree when `mode = "worktree"`).
- `POST /api/chats/{id}/messages` — Submits a user message and executes a bounded agent runtime turn.
- `GET /api/chats/{id}/export` — Exports a complete chat session (`chat`, `messages`, and `message_parts`) as a portable JSON bundle.
- `GET /api/chats/{id}/messages` — Returns ordered `messages` + nested `message_parts` for chat history hydration.
- `PATCH /api/chats/{id}` — Updates chat `title` and sets `title_user_set = 1`.
- `GET /api/worktrees/{id}/git-status` — Inspects the Git status of an isolated chat worktree.
- `GET /api/worktrees/{id}/diff` — Generates a bounded unified diff for an isolated chat worktree.
- `POST /api/worktrees/{id}/merge` — Merges an active worktree branch back into the project repository (`git merge --no-ff`) after explicit user confirmation (`confirm = true`).
- `POST /api/worktrees/{id}/discard` — Removes an isolated worktree directory and deletes its branch after explicit user confirmation (`confirm = true`).
- `POST /api/runs/{id}/cancel` — Cancels an active run and records a `run.cancelled` event.
- `GET /api/runs/{id}/events` — Streams ordered `run_events` via Server-Sent Events (SSE) with `Last-Event-ID` / `after_seq` replay support.
- `GET /api/approvals/pending` — Lists all pending tool-call approval requests.
- `POST /api/approvals/{id}/decide` — Records an approval/denial decision, optionally remembers a `command_rules` entry, and appends an `audit_logs` record.
- `GET /api/providers` — Lists configured model providers and credential presence status (`has_configured_secret`) without exposing secret values.
- `POST /api/providers` — Registers a model provider, validates its `base_url`, and stores its credential in the `CredentialStore` by `secret_ref`.
- `GET /api/providers/{id}/models` — Lists enabled models for a provider.
- `POST /api/providers/{id}/models` — Registers a model under a provider.
- `GET /api/extensions` — Lists enabled extensions.
- `GET /api/extensions/inventory` — Lists every extension with tool counts plus the workspace active-tool budget (`RECOMMENDED_MAX_ACTIVE_TOOLS = 25`).
- `PATCH /api/extensions/{id}` — Enables or disables an extension.

### Settings, onboarding & diagnostics UI
- First-run onboarding card appears when no project or provider is configured and can be skipped.
- `Cmd/Ctrl + ,` opens Settings with Providers & Models, Permissions & Rules, Extensions, Appearance, and Diagnostics tabs.
- The composer model selector persists `active_model_key` in `app_settings` and falls back to `local-mock` when no remote model is configured.
- API keys are submitted once, cleared from the form immediately, stored in the server `CredentialStore`, and surfaced only as `has_configured_secret: bool`.

---

## Contributing & Community

Contributions are welcome! Please review the following documents before opening an issue or submitting a pull request:

- [Contributing Guide (CONTRIBUTING.md)](./CONTRIBUTING.md)
- [Code of Conduct (CODE_OF_CONDUCT.md)](./CODE_OF_CONDUCT.md)
- [Security Policy (SECURITY.md)](./SECURITY.md)

---

## License

Licensed under the **Apache License, Version 2.0**. See the [`LICENSE`](./LICENSE) file for full details.