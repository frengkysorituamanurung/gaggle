# Gaggle — Roadmap & Milestone Plan

Inspired by **Goose** (Block's open-source Rust AI agent) and adapted to Gaggle's local-first, project-centric architecture.

---

## Current State (Milestones 1–7 Complete)

| Milestone | Status | Summary |
| :--- | :--- | :--- |
| M1 — Workspace Security & Run Recovery | ✅ Done | `WorkspacePathPolicy`, `RunEngine` crash recovery, `gaggle doctor`, `/api/diagnostics` |
| M2 — Filesystem & Git Inspection | ✅ Done | `fs_list`, `fs_read`, `fs_search`, `git_status`, `git_diff`, `/api/projects/{id}/files`, `/git-status`, `/diff` |
| M3 — Shell Execution & Stateful Approvals | ✅ Done | `fs_write` with diff recording, `shell_exec` (sanitized env, timeout, output cap), `decide_approval`, `verify_approved_execution`, `/api/approvals/*` |
| M4 — Provider Abstraction & Credentials | ✅ Done | `ModelProviderAdapter`, `OpenAiCompatibleAdapter`, `MockProviderAdapter`, `CredentialStore` / `SecretString`, `/api/providers`, `/api/providers/{id}/models` |
| M5 — Bounded Agent Runtime & SSE | ✅ Done | `AgentRuntime`, `BuiltinToolExecutor`, `CancellationToken`, `ActiveRunRegistry`, `POST /api/chats/{id}/messages`, `POST /api/runs/{id}/cancel`, `GET /api/runs/{id}/events` (SSE replay) |
| M6 — 3-Panel UI & Design Tokens | ✅ Done | Sidebar, Main Workspace, Contextual Details Panel, light/dark theme tokens, ARIA accessibility |
| M7 — Interactive UI Chat & Approvals | ✅ Done | Client-side SSE streaming, inline approval cards, tool call status badges, composer with `Cmd/Ctrl+Enter` |

**Quality baseline**: `cargo fmt --all -- --check` ✅, `cargo clippy --workspace --all-targets -- -D warnings` ✅, `cargo test --workspace` ✅ (15 tests across 6 test binaries).

---

## Milestone 8 — Git Worktree Isolation (Issue #11)

**Goal**: Isolate each chat's file changes in a dedicated Git worktree so the main project workspace remains untouched until the user explicitly merges or discards.

### User Problem
When the agent modifies files during a chat, those changes are written directly into the user's project directory. There is no way to review, merge, or discard them atomically per conversation.

### Key User Flows
1. User creates a chat with `mode = "worktree"`.
2. Gaggle creates a Git worktree on a new branch (`gaggle/chat-<id>`) under a configurable parent directory.
3. All tool execution (`fs.*`, `shell.exec`) inside that chat operates on the worktree path.
4. User reviews accumulated diffs in the Changes panel.
5. User triggers **Merge** (merges worktree branch into default branch) or **Discard** (removes worktree + deletes branch).

### Architecture & Security
- `WorktreeManager` in `gaggle-core` or `gaggle-tools` using argument-based `Command::new("git")` (`git worktree add`, `git worktree remove`, `git merge`, `git branch -D`).
- Worktree path validated via `WorkspacePathPolicy` (no symlink escapes, no sensitive paths).
- Reject worktree creation on non-Git repositories (`vcs_kind = 'none'`).
- Destructive operations (merge, discard) require explicit user confirmation via approval or UI action.
- Record all lifecycle events in `audit_logs`.

### Acceptance Criteria
- [ ] `git worktree add` creates an isolated worktree and records a `worktrees` row with `status = 'active'`.
- [ ] `chats.worktree_id` is set and tool execution uses the worktree path as `root`.
- [ ] `git_status` and `git_diff` operate on the worktree path.
- [ ] Merge endpoint runs `git merge --no-ff` from the project root and updates `status = 'merged'`.
- [ ] Discard endpoint runs `git worktree remove --force` and `git branch -D`, updating `status = 'discarded'`.
- [ ] Integration tests with temporary Git repos cover creation, isolation, merge, discard, and cleanup.
- [ ] Security tests for path traversal and symlink escape on worktree paths.

### Non-Goals
- No automatic merge without user confirmation.
- No support for non-Git VCS.

---

## Milestone 9 — MCP Client Transport (Issue #12)

**Goal**: Enable Gaggle to connect to external Model Context Protocol (MCP) servers over `stdio`, `streamable_http`, and `sse` transports, discovering and executing their tools through the central permission engine.

### User Problem
Gaggle's `extensions` table supports multiple transport kinds, but there is no MCP client implementation. Users cannot extend Gaggle's capabilities with external tools.

### Key User Flows
1. User configures an MCP server via `POST /api/extensions` with `kind`, `command`/`args` (stdio) or `url` (HTTP/SSE), and `env_secret_refs`.
2. On chat creation or when explicitly enabled, Gaggle performs the MCP `initialize` handshake and discovers tools via `tools/list`.
3. Discovered tools are upserted into `extension_tools` with their `input_schema` and `is_read_only` annotation.
4. When the agent proposes an MCP tool call, it is routed through `PermissionEngine` and executed via the MCP client's `tools/call` JSON-RPC request.
5. User can enable, disable, or uninstall extensions with full audit logging.

### Architecture & Security
- `McpClient` in `gaggle-mcp` implementing JSON-RPC 2.0 over three transports:
  - **stdio**: `Command::new(command).args(args)` with stdin/stdout line-based JSON-RPC, wall-clock timeout, and process-tree cancellation.
  - **streamable_http**: HTTP POST with `Content-Type: application/json`, parsing JSON or SSE responses.
  - **sse**: Establish SSE connection, send requests via HTTP POST, receive responses via SSE stream.
- Controlled environment variables (no host secrets inherited).
- SSRF protection for HTTP/SSE transports (validate schemes, hosts, redirects, destination scope).
- Extension capability grants (`extension_permissions`) must be explicitly granted before tool execution.

### Acceptance Criteria
- [ ] `stdio` transport spawns subprocess and communicates via JSON-RPC lines with timeout and cancellation.
- [ ] `streamable_http` and `sse` transports handle JSON-RPC over HTTP with SSRF protection.
- [ ] MCP `initialize` handshake validates server capabilities.
- [ ] `tools/list` populates `extension_tools` table.
- [ ] `tools/call` routes through `PermissionEngine` with `extension_id` binding.
- [ ] Extension lifecycle (install, enable, disable, uninstall) recorded in `audit_logs`.
- [ ] Extension failures handled gracefully (reported as `tool_calls.status = 'error'`).
- [ ] Cross-platform tests for stdio (macOS + Linux) and HTTP (mock server) transports.

### Non-Goals
- No MCP resources or prompts (tools only).
- No automatic installation from remote registries.
- No WASM runtime (`wasmtime`) in this milestone.

---

## Milestone 10 — Recipe Cron Scheduler (Issue #13)

**Goal**: Enable reusable agent task templates (recipes) with parameterized prompts and automatic cron-based scheduling.

### User Problem
Gaggle has database tables for `recipes`, `recipe_parameters`, `recipe_sub_recipes`, `recipe_extensions`, and `schedules`, but no runtime. Users cannot define reusable task templates or schedule them to run automatically.

### Key User Flows
1. User creates a recipe via `POST /api/recipes` with `name`, `title`, `instructions` (template with `{{ parameter_name }}`), `prompt`, `parameters`, and `extensions`.
2. User validates parameters: required parameters must be provided; optional parameters fall back to defaults.
3. User creates a schedule via `POST /api/schedules` with `recipe_id`, `project_id`, `cron_expr`, `timezone`, and `param_values`.
4. The background scheduler loop checks `schedules` every 60 seconds for due jobs (`enabled = 1 AND next_run_at <= now`), creates a scheduled chat, and computes the next `next_run_at`.
5. Sub-recipes: when a recipe calls a sub-recipe, a child chat (`chat_type = 'subagent'`) is created with `parent_tool_call_id` binding.

### Architecture & Security
- Recipe parser supporting inline JSON, YAML files, and URLs (`serde_yaml_ng`).
- Template rendering via `minijinja` with `{{ parameter_name }}` syntax.
- `croner` for cron expression evaluation with timezone support.
- Sub-recipe recursion guard to prevent infinite loops.
- Recipe instructions treated as untrusted content (prompt injection defense).

### Acceptance Criteria
- [ ] Recipe definitions load from inline JSON, YAML files, or URLs.
- [ ] Required parameters without defaults rejected at validation; optional parameters without defaults rejected.
- [ ] `{{ parameter_name }}` templates render correctly.
- [ ] Recipe execution creates a chat with `recipe_id` and `recipe_values` populated.
- [ ] Required extensions activated in `chat_extensions`.
- [ ] Cron scheduler loop checks `schedules` every 60 seconds and triggers due recipes.
- [ ] `next_run_at` computed correctly from `cron_expr` and `timezone`.
- [ ] Sub-recipe execution creates child chat with `chat_type = 'subagent'` and `parent_tool_call_id`.
- [ ] `sequential_when_repeated` forces concurrent sub-recipe calls to run sequentially.
- [ ] Scheduler handles missed runs (computes next future `next_run_at`).

### Non-Goals
- No visual recipe editor (API and file-based only).
- No interval-based or event-triggered scheduling (cron only).

---

## Milestone 11 — PTY Terminal Sessions (Issue #14)

**Goal**: Provide interactive terminal sessions in the browser via PTY, allowing users to run commands directly in their project workspace.

### User Problem
Gaggle has a `terminal_sessions` table but no PTY implementation. Users cannot open an interactive terminal to run commands, inspect output, or debug issues directly.

### Key User Flows
1. User opens a terminal panel in the UI, calling `POST /api/terminal-sessions` with `project_id`, `cwd`, `shell`, `cols`, and `rows`.
2. Gaggle spawns a PTY with the configured shell in `cwd`, stores the `pid`, and begins streaming output via WebSocket.
3. User keystrokes are forwarded to the PTY master; terminal resize updates PTY dimensions.
4. On shell exit, `terminal_sessions.status` is set to `exited`, `exit_code` is recorded, and `ended_at` is set.
5. On server shutdown, all running terminal sessions are terminated gracefully.

### Architecture & Security
- `portable-pty` for cross-platform PTY support.
- WebSocket for bidirectional I/O streaming (stdin, stdout, stderr).
- Scrollback stored in memory (not in SQLite); only session metadata persisted.
- Controlled environment variables (no host secrets inherited).
- Process-tree cancellation on session close or timeout.
- `WorkspacePathPolicy` validation for `cwd`.

### Acceptance Criteria
- [ ] PTY spawns successfully on macOS and Linux with configured shell.
- [ ] Terminal output streamed to browser in real-time (low latency).
- [ ] User input forwarded to PTY master.
- [ ] Terminal resize updates PTY dimensions and database record.
- [ ] Session exit detected and recorded (`status = 'exited'`, `exit_code`, `ended_at`).
- [ ] Scrollback stored in memory, not persisted to SQLite.
- [ ] Process-tree cancellation terminates shell and child processes on close.
- [ ] Wall-clock or idle timeout can terminate long-running sessions.
- [ ] Terminal sessions attachable to a chat or project.

### Non-Goals
- No terminal multiplexer (tmux/screen) integration.
- No terminal recording/replay.
- No shared terminal sessions (multi-user).

---

## Milestone 12 — Browser Sandbox (Issue #15)

**Goal**: Enable headless Chromium browser automation for agent tooling (navigation, clicking, typing, screenshots, content extraction).

### User Problem
Gaggle has `browser_sessions` and `browser_actions` tables but no browser implementation. The agent cannot interact with web pages for testing, scraping, or verification.

### Key User Flows
1. Agent calls `browser.navigate` $\rightarrow$ Gaggle creates a `browser_sessions` row, launches headless Chromium with isolated `profile_dir`, navigates to URL.
2. Agent calls `browser.click`, `browser.type`, `browser.scroll`, `browser.screenshot`, `browser.extract` $\rightarrow$ each action recorded in `browser_actions` with sequential `seq`.
3. `screenshot` captures PNG, stores as `attachment` (with `sha256` and `storage_path`), links via `screenshot_attachment_id`.
4. `extract` returns page text/HTML/structured data as JSON in `result`.
5. On completion or timeout, browser instance closed, `status = 'closed'`, `ended_at` recorded.

### Architecture & Security
- `chromiumoxide` (Chrome DevTools Protocol) for headless Chromium control.
- Isolated browser profiles per session (`profile_dir`) to prevent cookie/state leakage.
- SSRF protection: block `file://`, `javascript:`, and other dangerous schemes; validate destination URLs.
- Browser tool calls route through `PermissionEngine` (approval required in `ask` mode).
- Browser instances terminated on session close or server shutdown.

### Acceptance Criteria
- [ ] Headless Chromium launches on macOS and Linux.
- [ ] `navigate` loads URL and records final URL (after redirects).
- [ ] `click` clicks element by CSS selector and waits for load.
- [ ] `type` types text into input element by CSS selector.
- [ ] `scroll` scrolls page by amount or to element.
- [ ] `screenshot` captures PNG, stores as `attachment`, links via `screenshot_attachment_id`.
- [ ] `extract` returns page text or HTML as JSON in `result`.
- [ ] Every action recorded in `browser_actions` with sequential `seq`.
- [ ] Browser sessions isolated per chat (separate `profile_dir`).
- [ ] Browser tool calls route through `PermissionEngine`.
- [ ] Browser instances terminated on close or server shutdown.
- [ ] Page load timeouts and navigation errors handled gracefully.

### Non-Goals
- No browser extension (Chrome/Firefox) integration.
- No visual browser automation recording/replay.
- No multi-tab or multi-window support.

---

## Milestone 13 — Context Compaction & Chat Summarization

**Goal**: Implement automatic context compaction when conversations approach the model's context window limit, preserving critical constraints and unresolved approvals.

### User Problem
Long conversations eventually exceed the model's context window, causing failures or truncated responses. Gaggle has `chat_summaries` and `chats.compact_threshold` (default 0.75) but no compaction logic.

### Key User Flows
1. Before each model turn, `AgentRuntime` checks if `total_tokens / context_window >= compact_threshold`.
2. If threshold exceeded, a compaction pass summarizes older messages into a `chat_summaries` entry (`covers_until_seq`, `tokens_before`, `tokens_after`, `trigger = 'auto'`).
3. The summarized messages are replaced with the summary in the conversation context sent to the model.
4. User can manually trigger compaction via `POST /api/chats/{id}/compact`.

### Architecture & Security
- `CompactionEngine` in `gaggle-core` using the active provider to generate summaries.
- Preserve system prompts, unresolved approvals, and critical constraints during compaction.
- Treat compacted history as trusted (generated by the agent) but verify it doesn't drop security-relevant context.

### Acceptance Criteria
- [ ] Auto-compaction triggers when token ratio exceeds `compact_threshold`.
- [ ] `chat_summaries` entry created with correct `covers_until_seq`, `tokens_before`, `tokens_after`.
- [ ] Summarized messages replaced with summary in model context.
- [ ] Manual compaction endpoint (`POST /api/chats/{id}/compact`) works.
- [ ] Unresolved approvals and critical constraints preserved during compaction.
- [ ] Compaction does not silently drop security-relevant context.

### Non-Goals
- No native provider compaction (local summaries only).
- No cross-chat compaction (per-chat only).

---

## Milestone 14 — Memory System & Semantic Search

**Goal**: Implement project and global memory storage with FTS5 full-text search and optional vector embeddings for semantic retrieval.

### User Problem
Gaggle has `memories`, `memory_tags`, and `memory_embeddings` tables, plus FTS5 virtual tables, but no runtime implementation. The agent cannot store, retrieve, or search memories.

### Key User Flows
1. Agent calls `memory.write` $\rightarrow$ creates a `memories` row with `kind` (`fact`, `preference`, `decision`, `todo`, `summary`), `title`, `content`, optional `project_id`, and `source_chat_id` / `source_run_id`.
2. Agent calls `memory.read` $\rightarrow$ searches `memories_fts` using FTS5 `MATCH` query, returns ranked results.
3. User can pin memories (`pinned = 1`) to prioritize them in context assembly.
4. Optional: generate vector embeddings via an embedding model and store in `memory_embeddings` for semantic search.
5. User can view, edit, and delete memories via `GET/PUT/DELETE /api/memories`.

### Architecture & Security
- `MemoryRepository` in `gaggle-db` with `create`, `search_fts`, `list_by_project`, `update`, `soft_delete`.
- Memory content treated as untrusted (may contain prompt injection attempts).
- Embeddings generated only when an embedding model is configured.
- Memory export to `.pal/memory/*.md` for Git sharing (optional).

### Acceptance Criteria
- [ ] `memory.write` creates a memory with correct `kind`, `title`, `content`, and optional `project_id`.
- [ ] `memory.read` searches `memories_fts` and returns ranked results.
- [ ] Pinned memories are prioritized in context assembly.
- [ ] Soft delete (`deleted_at`) excludes memories from search results.
- [ ] Memory CRUD endpoints (`GET/PUT/DELETE /api/memories`) work.
- [ ] Optional vector embedding generation and semantic search.
- [ ] Memory export to `.pal/memory/*.md` (optional).

### Non-Goals
- No cross-device memory sync.
- No automatic memory extraction from conversations (manual or agent-initiated only).

---

## Milestone 15 — Multi-Provider Support & Streaming

**Goal**: Expand provider support beyond OpenAI-compatible to include Anthropic (native), ChatGPT (OAuth), Ollama (local), and additional providers with streaming SSE.

### User Problem
Gaggle currently only has `OpenAiCompatibleAdapter`. Users who want to use Anthropic (Claude), ChatGPT (OAuth sign-in), or local Ollama models cannot do so.

### Key User Flows
1. User configures an Anthropic provider $\rightarrow$ `AnthropicAdapter` handles `x-api-key` auth and `anthropic-version` header, streaming via SSE.
2. User configures ChatGPT $\rightarrow$ OAuth flow via `oauth2` crate, token refresh, and ChatGPT-compatible API.
3. User configures Ollama $\rightarrow$ local endpoint, no auth, streaming via NDJSON.
4. All providers normalize to `ModelCompletionResponse` with `text`, `tool_calls`, and `usage`.
5. Streaming responses are consumed by `AgentRuntime` via `Stream<Item = Result<...>>` instead of blocking `complete()`.

### Architecture & Security
- Provider-specific adapters implementing `ModelProviderAdapter` trait (or a new `StreamingModelProviderAdapter`).
- OAuth tokens stored in OS credential store via `secret_ref`.
- Provider URL validation and SSRF protection.
- Streaming cancellation propagation via `CancellationToken`.

### Acceptance Criteria
- [ ] `AnthropicAdapter` handles Claude models with `x-api-key` auth and SSE streaming.
- [ ] `ChatGptAdapter` implements OAuth sign-in flow and token refresh.
- [ ] `OllamaAdapter` handles local models without auth and NDJSON streaming.
- [ ] All adapters normalize to `ModelCompletionResponse`.
- [ ] Streaming responses consumed by `AgentRuntime` (not blocking `complete()`).
- [ ] Provider-specific error mapping (rate limits, auth failures, malformed responses).
- [ ] Cross-platform tests with mock servers for each adapter.

### Non-Goals
- No local model inference (Ollama is external).
- No provider-specific tool calling differences (all normalize to `ProposedToolCall`).

---

## Milestone 16 — Settings, First-Run Experience & Diagnostics UI

**Goal**: Implement a polished settings screen, first-run onboarding wizard, and diagnostics view in the UI.

### User Problem
Gaggle has backend endpoints for diagnostics, providers, projects, and approvals, but no settings UI. Users cannot configure providers, manage permissions, or view diagnostics from the browser.

### Key User Flows
1. **First-Run**: On first launch, show a welcome screen explaining Gaggle, guide user to configure an AI provider, and add an existing project.
2. **Settings Screen**: Provider/model configuration, credential status (without revealing values), permission defaults, theme selection, data/diagnostic settings.
3. **Diagnostics View**: Display `gaggle doctor` output (OS, arch, SQLite health, migrations, active projects, pending approvals) in a formatted panel.
4. **Permission Management**: View and modify `tool_permissions` and `command_rules` per project.

### Architecture & Security
- Settings UI calls existing backend endpoints (`/api/providers`, `/api/diagnostics`, `/api/projects`).
- Credential values never displayed in the UI (only `has_configured_secret` boolean).
- Permission changes persisted via `PermissionRepository`.

### Acceptance Criteria
- [ ] First-run welcome screen with provider setup and project registration.
- [ ] Settings screen with provider/model configuration.
- [ ] Credential status displayed without revealing values.
- [ ] Permission defaults configurable per project.
- [ ] Theme selection (light/dark/system).
- [ ] Diagnostics view showing `gaggle doctor` output.
- [ ] Permission management UI (view/modify `tool_permissions` and `command_rules`).

### Non-Goals
- No advanced settings (experimental features) in this milestone.
- No multi-user or team settings.

---

## Milestone 17 — Context Assembly & Project Instructions

**Goal**: Implement context assembly for agent turns, loading project instructions, `AGENTS.md`, memories, and recent conversation history within token budget limits.

### User Problem
`AgentRuntime` currently sends only the user prompt as context. It doesn't load project instructions (`project_settings.instructions`), `AGENTS.md` files, relevant memories, or recent conversation history.

### Key User Flows
1. Before each model turn, `AgentRuntime` assembles context:
   - System prompt with project instructions and `AGENTS.md` content.
   - Recent conversation history (bounded by token budget).
   - Relevant memories (pinned + FTS5 search results).
   - Tool definitions from enabled extensions.
2. Context is bounded by `context_window` and `compact_threshold`.
3. `AGENTS.md` files are discovered from the project root and nested directories.

### Architecture & Security
- `ContextAssembler` in `gaggle-core` with token budget management.
- Project instructions treated as trusted (user-authored).
- Repository files and tool outputs treated as untrusted (prompt injection defense).
- Secrets excluded from context assembly.

### Acceptance Criteria
- [ ] Project instructions (`project_settings.instructions`) loaded into system prompt.
- [ ] `AGENTS.md` discovered and loaded from project root and nested directories.
- [ ] Recent conversation history included within token budget.
- [ ] Pinned memories and FTS5 search results included.
- [ ] Tool definitions from enabled extensions included.
- [ ] Context bounded by `context_window` and `compact_threshold`.
- [ ] Secrets excluded from context.

### Non-Goals
- No automatic memory extraction from conversations.
- No cross-project context sharing.

---

## Milestone 18 — Subagents & Parallel Execution

**Goal**: Enable subagent spawning for parallel task execution, with parent-child chat relationships and inherited permissions.

### User Problem
Gaggle supports `chats.parent_chat_id` and `chats.parent_tool_call_id` for subagent tracking, but there is no runtime implementation for spawning and managing subagents.

### Key User Flows
1. Agent calls a sub-recipe or delegates a task $\rightarrow$ creates a child chat (`chat_type = 'subagent'`) with `parent_chat_id` and `parent_tool_call_id`.
2. Child chat runs independently with its own conversation history and tool permissions.
3. Parent chat waits for child completion and receives a summary result.
4. `sequential_when_repeated` forces concurrent sub-recipe calls to run sequentially.

### Architecture & Security
- `SubagentManager` in `gaggle-core` with bounded concurrency (`max_parallel_subagents`).
- Child chats inherit parent's `permission_mode` but can be overridden.
- Subagent results summarized and fed back to parent as tool results.

### Acceptance Criteria
- [ ] Subagent chat created with `chat_type = 'subagent'` and `parent_chat_id` / `parent_tool_call_id`.
- [ ] Subagent runs independently with its own conversation history.
- [ ] Parent waits for subagent completion and receives summary.
- [ ] `sequential_when_repeated` enforces sequential execution.
- [ ] Bounded concurrency prevents resource exhaustion.
- [ ] Subagent cancellation propagates to parent.

### Non-Goals
- No nested subagents beyond depth 1 (configurable `subagent_depth`).
- No shared state between parent and child (isolated conversations).

---

## Milestone 19 — Export, Import & Backup

**Goal**: Implement chat export/import as JSON, database backup/restore, and project memory export to `.pal/memory/*.md`.

### User Problem
Gaggle has `export_chat` in `ChatRepository` but no import, backup, or restore functionality. Users cannot migrate data between devices or recover from corruption.

### Key User Flows
1. `gaggle export chat <id> --output chat.json` exports a full chat bundle.
2. `gaggle import chat chat.json --project <id>` imports a chat bundle into a project.
3. `gaggle backup --output backup.tar.gz` creates a verified backup of the database and attachments.
4. `gaggle restore backup.tar.gz` restores from a backup with verification.
5. Memory export to `.pal/memory/*.md` for Git sharing.

### Architecture & Security
- `ChatImportBundle` validation (schema version, FK integrity).
- Backup includes SQLite DB (WAL checkpoint), attachments directory, and manifest with checksums.
- Restore verifies checksums before replacing existing data.
- Never replace an existing directory without explicit confirmation.

### Acceptance Criteria
- [ ] Chat export produces valid JSON bundle with `chat`, `messages`, and `message_parts`.
- [ ] Chat import validates bundle and creates chat in target project.
- [ ] Database backup creates checksummed archive.
- [ ] Database restore verifies checksums and restores atomically.
- [ ] Memory export to `.pal/memory/*.md`.
- [ ] Cross-platform backup/restore (macOS + Linux).

### Non-Goals
- No cloud backup or sync.
- No incremental backups (full backup only).

---

## Milestone 20 — Observability, Diagnostics & Error Reporting

**Goal**: Implement structured tracing, redacted diagnostics export, and actionable error reporting across the application.

### User Problem
Gaggle has `tracing` in dependencies but no structured logging, correlation IDs, or diagnostic export. Users cannot diagnose failures or share redacted diagnostics.

### Key User Flows
1. All log entries include `chat_id`, `run_id`, and `tool_call_id` correlation fields.
2. Secrets and sensitive arguments are redacted before serialization.
3. `gaggle doctor` reports build identity, core health, and why optional capabilities are unavailable.
4. Redacted diagnostics export available in Settings and in chat problem notices.
5. Error messages are actionable (what failed, which operation, whether data was preserved, retry guidance).

### Architecture & Security
- `tracing-subscriber` with `EnvFilter` and structured fields.
- Correlation IDs propagated through `AgentRuntime`, `RunRepository`, and `PermissionEngine`.
- Redaction applied at the `tracing` layer (not in individual log calls).
- Diagnostic export requires explicit user action (no automatic telemetry).

### Acceptance Criteria
- [ ] Structured tracing with correlation IDs for chat, run, and tool call.
- [ ] Secrets and sensitive arguments redacted before serialization.
- [ ] `gaggle doctor` reports build identity, core health, and optional capability status.
- [ ] Redacted diagnostics export available in Settings.
- [ ] Error messages include failure category, affected operation, data preservation status, and retry guidance.
- [ ] No full prompts, source files, or raw command output in default logs.

### Non-Goals
- No automatic telemetry or crash reporting.
- No remote log aggregation.

---

## Summary: Goose-Inspired Capability Mapping

| Goose Concept | Gaggle Equivalent | Milestone |
| :--- | :--- | :--- |
| Session (working dir, type, recipe, provider/model, tokens) | `chats` table | ✅ Done (M1–M7) |
| SessionType (User, SubAgent, Scheduled, Hidden, Terminal) | `chats.chat_type` | ✅ Done (M1–M7) |
| Mode (Autonomous, Manual Approval, Smart Approval, Chat Only) | `chats.permission_mode` | ✅ Done (M1–M7) |
| Per-tool permissions (Always Allow, Ask Before, Never Allow) | `tool_permissions` | ✅ Done (M1–M7) |
| Extension (builtin, stdio, streamable HTTP, SSE, WASM) | `extensions` + `extension_tools` | M9 (MCP Transport) |
| Extension active per session | `chat_extensions` | M9 (MCP Transport) |
| Recipe (instructions, params, extensions, sub-recipes) | `recipes` + children | M10 (Recipe Scheduler) |
| Scheduler (cron runs recipe) | `schedules` | M10 (Recipe Scheduler) |
| Context compaction (0.75 threshold) | `chat_summaries` + `compact_threshold` | M13 (Context Compaction) |
| AI-generated session name (`title_user_set`) | `chats.title_user_set` | ✅ Done (M1–M7) |
| Token counting (total + accumulated) | `chats` + `usage_records` | ✅ Done (M5) |
| Schema version + migrations + JSONL import | `sqlx::migrate!` + `_sqlx_migrations` | ✅ Done (M1) |
| Session export/import as JSON | `/api/chats/{id}/export` | M19 (Export/Import/Backup) |
| Git Worktree mode | `worktrees` table | M8 (Git Worktree Isolation) |
| Changes panel (diff + review comments) | `file_changes` + `review_comments` | M8 (Git Worktree Isolation) |
| Durable run (events + checkpoints) | `runs` + `run_events` + `run_checkpoints` | ✅ Done (M5) |
| Terminal (PTY) | `terminal_sessions` | M11 (PTY Terminal Sessions) |
| Browser sandbox (CDP) | `browser_sessions` + `browser_actions` | M12 (Browser Sandbox) |
| Subagent / sub-recipe (child chat) | `chats.parent_chat_id` + `parent_tool_call_id` | M18 (Subagents & Parallel Execution) |
| Memory (fact, preference, decision, todo, summary) | `memories` + `memory_tags` + `memory_embeddings` | M14 (Memory System & Semantic Search) |
| Project as first-class entity | `projects` table | ✅ Done (M1) |
| Device pairing (mobile browser to local server) | `devices` table | M16 (Settings & Diagnostics UI) |
| Audit log | `audit_logs` table | ✅ Done (M3) |
| Tool risk cache (LLM judge) | `tool_risk_cache` | ✅ Done (M1) |
EOF
