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
- **Multi-Provider Streaming**: Normalized adapters for OpenAI-compatible APIs, Anthropic (SSE), local Ollama (NDJSON), and ChatGPT OAuth + PKCE. `AgentRuntime` consumes native streams with tool calls and usage reporting.
- **Bounded Context Assembly**: Project instructions, nested `AGENTS.md`, recent history, pinned/FTS memories, and enabled tool definitions are assembled within the model context budget with secret redaction.
- **Credential References Only**: API keys and OAuth tokens are **never** stored in SQLite. The database stores only a `secret_ref` identifier resolved through the `CredentialStore` abstraction (currently an in-memory store; OS keychain integration is planned).

---

## Workspace Architecture

Gaggle is organized as a multi-crate Cargo workspace with clear domain boundaries:

```text
gaggle/
├─ Cargo.toml                # Cargo workspace manifest
├─ rust-toolchain.toml       # Stable toolchain, rustfmt, and Clippy
├─ LICENSE                   # Apache-2.0 License
├─ README.md                 # Project overview and usage guide
├─ CONTRIBUTING.md           # Contribution guidelines
├─ CODE_OF_CONDUCT.md        # Community Code of Conduct
├─ SECURITY.md               # Security policy and vulnerability reporting
├─ assets/
│  └─ Gaggle.icns            # macOS application icon
├─ docs/
│  ├─ ERD.md                 # Database schema and ERD documentation
│  └─ development/           # Data portability, CI/CD audit, and packaging documentation
├─ .github/
│  ├─ workflows/             # CI, package, release, and security workflows
│  └─ scripts/               # Reproducible packaging and validation scripts
└─ crates/
   ├─ gaggle-core/           # Agent loop, durable run recovery, compaction, 7-step permission engine, Code Mode
   ├─ gaggle-db/             # SQLite connection (WAL, FK), sqlx migrations, 37 ERD models, domain repositories
   ├─ gaggle-tools/          # Built-in extensions (fs, shell, memory, web, browser) and unified diff generation
   ├─ gaggle-mcp/            # MCP client integration, extension registry, active tool limit guard (~25 tools)
   ├─ gaggle-recipes/        # Recipe parser, required/optional parameter validation, minijinja templates, scheduler
   ├─ gaggle-server/         # Axum HTTP/SSE server, REST API, chat JSON export/import, embedded Web UI serving
   ├─ gaggle-cli/            # `gaggle` CLI and `gaggle-desktop` local launcher
   └─ gaggle-ui/             # Web UI shell served by gaggle-server
```

---

## Storage Design & Database Conventions

Gaggle uses a **single global SQLite database** across all projects. Storing the database globally prevents polluting project repositories with `.gaggle/` folders, supports read-only workspaces, avoids duplicating databases across Git worktrees, and preserves chat history if a project directory is moved or deleted.

Default database locations (resolved via the `directories` crate):
- **macOS**: `~/Library/Application Support/gaggle/gaggle.db`
- **Linux**: `~/.local/share/gaggle/gaggle.db`
- **Windows**: `%APPDATA%\gaggle\data\gaggle.db`

Attachments live beside the database in `<data directory>/attachments/`. See
[Data Portability](./docs/development/data-portability.md) for export, import,
memory sharing, checksummed backups, and safe restore procedures.

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

### Desktop Launcher and Installable Packages

Gaggle's UI is served by the local Axum server. The `gaggle-desktop` binary
starts that server and opens the default browser:

```bash
cargo run -p gaggle-cli --bin gaggle-desktop
```

The CI/CD pipeline builds native packages for pull requests targeting
`development` or `main`, and for pushes to those branches:

```text
Gaggle-macOS-arm64.zip
Gaggle-macOS-x64.zip
Gaggle-linux-x86_64.tar.gz
Gaggle-linux-aarch64.tar.gz
```

Each workflow artifact includes the package plus a SHA-256 file and JSON
metadata. Pull request builds are artifact-only. Stable releases are published
only from `v*` tags whose commits are on `main`. See
[Building and Packaging Gaggle](./docs/development/packaging.md) for
installation steps, runtime requirements, signing status, and the release
procedure. The macOS artifact is a ZIP containing `Gaggle.app`; no `.dmg`
installer is produced.

### Install on macOS (ZIP)

1. Download `Gaggle-macOS-arm64.zip` on Apple Silicon or
   `Gaggle-macOS-x64.zip` on Intel.
2. Verify the checksum next to the downloaded artifact:

   ```bash
   shasum -a 256 -c Gaggle-macOS-arm64.zip.sha256
   ```

3. Extract the bundle and move it into `/Applications`:

   ```bash
   ditto -x -k Gaggle-macOS-arm64.zip /Applications
   open /Applications/Gaggle.app
   ```

   `ditto` preserves the app bundle structure and executable permissions. Do
   not move the raw `Gaggle` executable out of the bundle.

4. Gaggle starts a local server on `127.0.0.1:4321` and opens the default
   browser. If the browser does not open, visit <http://127.0.0.1:4321>.

The database is stored outside the app bundle at
`~/Library/Application Support/gaggle/gaggle.db`. Replacing `Gaggle.app` does
not delete chats, settings, or memories.

### macOS Gatekeeper behavior

Release artifacts are not notarized unless the release workflow had access to
the Apple signing secrets. An unsigned or ad-hoc-signed build may show a
Gatekeeper warning such as "Gaggle.app cannot be opened because the developer
cannot be verified".

Use the standard confirmation flow instead of disabling Gatekeeper:

1. Open the app once from Finder, dismiss the warning, then open **System
   Settings → Privacy & Security**.
2. Click **Open Anyway** for Gaggle and confirm.
3. Alternatively, right-click `Gaggle.app` in Finder, choose **Open**, and
   confirm.

Do not run `spctl --master-disable`. If you manage downloads yourself and
accept the security trade-off, macOS may attach a quarantine attribute; the
standard Finder confirmation is still preferred over removing it manually.

### Reproducible macOS ZIP locally

The checked-in packaging script builds the release binaries, assembles
`Gaggle.app`, validates `Info.plist`, signs the bundle (ad-hoc by default),
recreates the ZIP with `ditto`, prints the SHA-256 checksum, and runs an
extraction plus `/api/health` smoke test:

```bash
./scripts/package_macos_zip.sh
```

Common options:

```bash
# Use a Developer ID identity instead of ad-hoc signing
./scripts/package_macos_zip.sh --sign-identity "Developer ID Application: Example (TEAMID)"

# Reuse prebuilt release binaries and skip the smoke test
./scripts/package_macos_zip.sh --binary-dir target/aarch64-apple-darwin/release --skip-smoke-test

# Build the other architecture
./scripts/package_macos_zip.sh --arch x64
```

Outputs are written to `dist/`:

```text
dist/Gaggle.app
dist/Gaggle-macOS-arm64.zip
dist/Gaggle-macOS-arm64.zip.sha256
dist/Gaggle-macOS-arm64.json
```

Passing `--skip-smoke-test` is only appropriate for local iteration; CI and
release validation always launch the extracted bundle and call `/api/health`.

### Build and run on Linux

```bash
cargo build --release --locked -p gaggle-cli --bin gaggle --bin gaggle-desktop
./target/release/gaggle-desktop
```

The launcher starts the local server and opens the default browser with
`xdg-open`. Use `--no-browser` on headless sessions and open
<http://127.0.0.1:4321> manually. The CLI is available as
`./target/release/gaggle` (`gaggle --help`).

Portable Linux archives can also be built locally:

```bash
./.github/scripts/package-linux.sh \
  --target x86_64-unknown-linux-gnu \
  --version 0.1.0 \
  --binary-dir "$PWD/target/x86_64-unknown-linux-gnu/release" \
  --output-dir "$PWD/dist" \
  --commit "$(git rev-parse HEAD)" \
  --ref "$(git rev-parse HEAD)"

tar -xzf dist/Gaggle-linux-x86_64.tar.gz
cd Gaggle-linux-x86_64
./install.sh
```

`install.sh` installs into the current user's `~/.local` directories without
`sudo`, including a desktop entry. Linux data lives at
`${XDG_DATA_HOME:-$HOME/.local/share}/gaggle/gaggle.db`.

### Keyboard shortcuts

| Shortcut | Action |
| :--- | :--- |
| `Cmd/Ctrl + K` | Open the quick switcher for projects and conversations. |
| `Cmd/Ctrl + N` | Create a new chat in the active project. |
| `Cmd/Ctrl + B` | Toggle the sidebar. |
| `Cmd/Ctrl + I` | Toggle the contextual details inspector. |
| `Cmd/Ctrl + ,` | Open Settings & Diagnostics. |
| `Cmd/Ctrl + /` | Open the in-app keyboard shortcut cheatsheet. |
| `Cmd/Ctrl + Enter` | Send the current composer message. |
| `Esc` | Close the open dialog and restore focus to its opener. |

Native editing shortcuts (`Cmd/Ctrl + C`, `V`, `X`, `A`, `Z`) are never
intercepted while focus is inside an input or textarea. Dialogs trap `Tab` and
`Shift + Tab` focus until they close.

---

## CLI Usage (`gaggle`)

The `gaggle` binary is provided by the `gaggle-cli` crate.

### 1. Print the Global Database Path
```bash
cargo run -p gaggle-cli -- db-path
```

### 2. Run Diagnostics & Database Health Check
```bash
cargo run -p gaggle-cli -- doctor          # human-readable summary
cargo run -p gaggle-cli -- doctor --json   # full machine-readable report
```
The report includes build identity (version, commit, profile, target), core health checks, optional
capability availability with reasons and next actions, and an explicit privacy statement. It never
contains prompts, source files, secrets, or raw tool output. See
[`docs/development/observability.md`](docs/development/observability.md).

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

### 6. Launch the Desktop Entry Point
```bash
cargo run -p gaggle-cli --bin gaggle-desktop -- --no-browser
```
The desktop launcher starts the same server and, unless `--no-browser` is used,
opens `http://127.0.0.1:4321` in the default browser.

### 7. Export a Chat Bundle
```bash
cargo run -p gaggle-cli -- export chat <chat-id> --output chat.json
```
Writes a versioned `gaggle.chat-bundle` JSON file containing `chat`, `messages`,
and `message_parts`, plus `format`, `version`, and `exported_at` metadata.

### 8. Import a Chat Bundle
```bash
cargo run -p gaggle-cli -- import chat chat.json --project <project-id>
```
Validates the bundle and creates a **new** chat in the target project. Existing
chats are never overwritten; run-tool-attachment references that are not part of
the bundle are dropped.

### 9. Export Project Memories
```bash
cargo run -p gaggle-cli -- export memory --project <project-id>
# Optional directory inside the project workspace:
cargo run -p gaggle-cli -- export memory --project <project-id> --output .pal/memory
```
Writes one redacted Markdown file per active memory to `<project>/.pal/memory/`
so project knowledge can be shared through Git.

### 10. Create a Checksummed Backup
```bash
cargo run -p gaggle-cli -- backup --output backup.tar.gz
```
Creates a `.tar.gz` containing a WAL-safe `VACUUM INTO` snapshot of `gaggle.db`,
the `attachments/` directory when present, and a `manifest.json` with SHA-256
checksums. The command prints the archive SHA-256 and refuses to overwrite an
existing file without `--force`.

### 11. Verify and Restore a Backup
```bash
cargo run -p gaggle-cli -- restore backup.tar.gz --dry-run
cargo run -p gaggle-cli -- restore backup.tar.gz --force
```
Restore verifies archive path safety, every SHA-256 checksum, and SQLite
integrity before touching existing data. Existing databases and attachment
directories are moved aside to `gaggle-restore-previous-<timestamp>/` and only
replaced after verification; without `--force` the command refuses to replace
anything. Stop the server before restoring.

On startup, Gaggle automatically:
1. Opens the global SQLite pool in WAL mode and applies any pending `sqlx` migrations.
2. Registers all bundled built-in extensions (`gaggle.fs`, `gaggle.shell`, `gaggle.memory`, `gaggle.web`, `gaggle.browser`).
3. Safely recovers any durable runs left in `running` or `waiting_approval` state by expiring stale approvals and transitioning interrupted runs to `paused`.

### Core HTTP Endpoints
- `GET /` — Serves the Gaggle Web UI.
- `GET /api/health` — Health check endpoint (`{"status": "ok", "service": "gaggle"}`).
- `GET /api/diagnostics` — Returns a redacted diagnostic report with build identity, core health checks, optional capability availability and reasons, and active counts.
- `GET /api/diagnostics/export` — Explicit, user-initiated redacted diagnostics export with up to ten recent failure summaries; contains no prompts, source files, tool arguments, or raw tool output.
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
- `POST /api/providers` — Registers a model provider (`openai`, `anthropic`, `chatgpt`, `openrouter`, `deepseek`, `ollama`, or `openai_compat`), validates its `base_url`, and stores credentials in the `CredentialStore` by `secret_ref`.
- `GET /api/providers/{id}/models` — Lists enabled models for a provider.
- `POST /api/providers/{id}/models` — Registers a model under a provider.
- `POST /api/providers/{id}/oauth/chatgpt/start` — Starts a ChatGPT OAuth 2.0 authorization-code + PKCE flow and returns the authorization URL and `state`.
- `POST /api/providers/{id}/oauth/chatgpt/exchange` — Exchanges an authorization code for OAuth tokens stored only in the `CredentialStore`.
- `GET /api/oauth/chatgpt/callback` — OAuth redirect target that completes the PKCE exchange and closes the sign-in window.
- `GET /api/memories` — Lists project + global memories, or runs sanitized FTS5 search when `query` is supplied.
- `POST /api/memories` — Creates a memory with `kind`, `title`, `content`, optional `tags`, and `pinned`.
- `PATCH /api/memories/{id}` — Updates memory content, kind, pin state, or tags.
- `DELETE /api/memories/{id}` — Soft-deletes a memory (`deleted_at`) so it is excluded from context and search.
- `GET /api/memories/export` — Returns a Markdown export of active project + global memories with secrets redacted.
- `GET /api/extensions` — Lists enabled extensions.
- `GET /api/extensions/inventory` — Lists every extension with tool counts plus the workspace active-tool budget (`RECOMMENDED_MAX_ACTIVE_TOOLS = 25`).
- `PATCH /api/extensions/{id}` — Enables or disables an extension.

### Settings, onboarding & diagnostics UI
- First-run onboarding card appears when no project or provider is configured and can be skipped.
- `Cmd/Ctrl + ,` opens Settings with Providers & Models, Permissions & Rules, Extensions, Memory, Appearance, and Diagnostics tabs.
- The composer model selector persists `active_model_key` in `app_settings` and falls back to `local-mock` when no remote model is configured.
- API keys are submitted once, cleared from the form immediately, stored in the server `CredentialStore`, and surfaced only as `has_configured_secret: bool`.
- ChatGPT providers can complete an OAuth + PKCE sign-in from Settings; access and refresh tokens remain in the credential store.
- The Memory tab creates, searches, pins, and soft-deletes project-scoped and global memories.
- The Diagnostics tab refreshes, copies, and downloads a redacted export on explicit user action; failed chat turns show a problem notice with failure category, retry guidance, and a diagnostics copy action. No telemetry is sent automatically.

### Context assembly, memory & providers
- `ContextAssembler` loads `project_settings.instructions`, bounded `AGENTS.md` files from the project root and nested directories, recent chat history, pinned and FTS-matched memories, and enabled extension tool definitions before each model turn.
- Context is bounded by the model's `context_window` and the chat's `compact_threshold`; secrets are redacted and repository/tool content is marked as untrusted.
- Native streaming is implemented for OpenAI-compatible, Anthropic (SSE), Ollama (NDJSON), and ChatGPT OAuth providers, with normalized `text`, `tool_calls`, and `usage` events consumed by `AgentRuntime`.
- Provider-specific errors map rate limits, authentication failures, timeouts, and malformed streams without logging credentials.

---

## Contributing & Community

Contributions are welcome! Please review the following documents before opening an issue or submitting a pull request:

- [Contributing Guide (CONTRIBUTING.md)](./CONTRIBUTING.md)
- [Code of Conduct (CODE_OF_CONDUCT.md)](./CODE_OF_CONDUCT.md)
- [Security Policy (SECURITY.md)](./SECURITY.md)

---

## License

Licensed under the **Apache License, Version 2.0**. See the [`LICENSE`](./LICENSE) file for full details.