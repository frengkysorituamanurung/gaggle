# Gaggle Project Instructions

These instructions are automatically loaded by OpenCode in every session within the **Gaggle** repository.

## 0. Local Execution Policy (Session Override)

- **Do not run any local build, test, lint, format, or other execution commands** (including `cargo fmt`, `cargo clippy`, `cargo check`, `cargo test`, `cargo build`, or scripts) in this repository.
- Work is limited to **editing code and files only**. Verification and quality gates run in CI, not locally.
- Use the provided GitHub token for all `gh`/GitHub operations; do not print or commit the token.
- This policy takes precedence over the quality-gate commands listed elsewhere in this document.

## 1. Mandatory Skill Routing & Discovery

Before planning, designing, or modifying any code in response to a user prompt, you **MUST** load the relevant canonical skill(s) from `.opencode/skills/<skill-id>/SKILL.md` using the `skill` tool:

| Domain / Task Area | Canonical Skill IDs (`.opencode/skills/<id>/SKILL.md`) |
| :--- | :--- |
| Requirements, scope, MVP boundaries, prioritization, user stories | `product-manager`, `research-thinking` |
| System boundaries, ADRs, durable execution, cross-crate design | `architecture`, `gaggle-architecture` |
| Rust implementation, error handling, async concurrency, traits | `rust-engineering` |
| Threat modeling, filesystem/path validation, shell safety, secrets, approvals | `security` |
| SQLite schema, `sqlx` migrations, transactions, FTS5, recovery | `database-sqlite`, `gaggle-db-schema` |
| Agent loop, tool-call lifecycle, streaming, cancellation, checkpoints | `agent-runtime`, `gaggle-architecture` |
| Model provider adapters, auth, streaming normalization, retries | `provider-integrations` |
| Desktop/Web UI layout, design system tokens, accessibility, UX states | `ui-ux` |
| Built-in tools, MCP client/extensions, custom skills, recipes | `skills-and-extensions` |
| Git status, diffs, patches, worktree lifecycle, protecting user work | `git-workflows` |
| Startup latency, memory bounds, macOS & Linux platform compatibility | `performance-cross-platform` |
| Structured `tracing`, redaction, metrics, actionable errors, diagnostics | `observability-reliability` |
| Unit, integration, E2E, security, and fault-injection testing | `testing-qa` |
| Documentation, ADRs, contributor guides, release & upgrade notes | `documentation-open-source` |
| Any code, docs, config, or release change | `ponytail` |

**Rule**: Do not skip loading the applicable skill(s) via the `skill` tool when working in its domain. For every change, also load and follow `ponytail`. Never invent skill contents.

---

## 2. Workspace Architecture (`gaggle`)

- `crates/gaggle-db`: Global SQLite persistence (`gaggle.db`), PRAGMAs (`WAL`, `foreign_keys=ON`, `synchronous=NORMAL`, `busy_timeout=5000`, `auto_vacuum=INCREMENTAL`), versioned `sqlx` migrations, 37 ERD models, FTS5 virtual tables (`memories_fts`, `messages_fts`), and domain repositories. Must not depend on other Gaggle crates.
- `crates/gaggle-core`: Domain types, 7-step `PermissionEngine`, bounded agent runtime & durable run recovery (`RunEngine`), context compaction, and Code Mode configuration.
- `crates/gaggle-tools`: Controlled filesystem, shell, memory, web, browser, and diff (`similar`) tools.
- `crates/gaggle-mcp`: Model Context Protocol (MCP) integration, extension registry, and active tool limit guard (`<= 25` tools per chat).
- `crates/gaggle-recipes`: Reusable recipe parser, parameter validation, `minijinja` templates, and `croner` scheduler.
- `crates/gaggle-server`: Local `axum` HTTP/SSE API, orchestration, chat JSON export/import, and UI hosting.
- `crates/gaggle-cli`: Command-line interface (`gaggle`) and diagnostics.
- `crates/gaggle-ui`: User interface and design system components.

---

## 3. Non-Negotiable Engineering & Security Constraints

1. **Security & User Control First**:
   - The LLM is never an authorization authority. Every tool call must be validated server-side immediately before execution.
   - Default permission mode is `ask`. `command_rules` with `action = 'deny'` are authoritative even in `full` mode.
   - Restrict filesystem operations to canonicalized, validated project roots (defend against `..` traversal and symlink escapes).
   - Never store plaintext API keys or OAuth tokens in SQLite, config files, or logs; store only `secret_ref` identifiers pointing to the OS credential store.
2. **Database Integrity**:
   - Use UUIDv7 (`TEXT`) for primary keys and Unix epoch milliseconds (`INTEGER`) for timestamps.
   - Enforce `CHECK(json_valid(...))` on JSON columns and `CHECK(col IN (...))` on enum/boolean columns.
   - Keep transactions short and never hold SQLite locks across network or model calls.
3. **Quality Gates & Language Policy**:
   - All code, comments, documentation, commit messages, and skills must be written in English.
   - Before reporting any task complete, run and report actual results for:
     - `cargo fmt --all -- --check`
     - `cargo clippy --workspace --all-targets -- -D warnings`
     - `cargo test --workspace`
