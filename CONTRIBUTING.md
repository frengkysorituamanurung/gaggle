# Contributing to Gaggle

Thank you for your interest in contributing to **Gaggle**! This document outlines our development workflow, crate architecture, SQLite schema conventions, and Rust coding standards to help get your contributions reviewed and merged smoothly.

By participating in this project, you agree to abide by our [Code of Conduct](./CODE_OF_CONDUCT.md).

---

## Table of Contents

1. [Ways to Contribute](#1-ways-to-contribute)
2. [Development Environment Setup](#2-development-environment-setup)
3. [Workspace Structure & Crate Responsibilities](#3-workspace-structure--crate-responsibilities)
4. [SQLite Schema & Migration Rules](#4-sqlite-schema--migration-rules)
5. [Rust Coding Standards](#5-rust-coding-standards)
6. [Testing Guidelines](#6-testing-guidelines)
7. [Pull Request Workflow & Commit Conventions](#7-pull-request-workflow--commit-conventions)
8. [Contribution License](#8-contribution-license)

---

## 1. Ways to Contribute

- **Report Bugs**: Open a GitHub Issue with clear reproduction steps, OS details, Rust version (`rustc --version`), and relevant logs.
- **Propose Features or Architecture Changes**: Open an issue to discuss significant changes before writing large patches, especially when modifying the SQLite schema (`gaggle-db`) or permission evaluation logic (`gaggle-core`).
- **Submit Pull Requests**: Fix bugs, add tests, improve performance, or enhance documentation.
- **Report Security Vulnerabilities**: Follow the private disclosure instructions in [SECURITY.md](./SECURITY.md).

---

## 2. Development Environment Setup

1. Ensure you have the latest **stable Rust toolchain** (supporting `edition = "2024"`), along with `rustfmt` and `clippy`:
   ```bash
   rustup update stable
   rustup component add rustfmt clippy
   ```

2. Clone the repository and verify that the workspace builds and all tests pass:
   ```bash
   git clone https://github.com/frengky/gaggle.git
   cd gaggle
   cargo build --workspace
   cargo fmt --all -- --check
   cargo clippy --workspace --all-targets --locked -- -D warnings
   cargo check --workspace --all-targets --locked
   cargo test --workspace --locked
   ```

---

## 3. Workspace Structure & Crate Responsibilities

Keep changes aligned with the domain boundaries of each crate:

| Crate | Responsibility |
| :--- | :--- |
| `crates/gaggle-db` | Global SQLite connection pool, PRAGMA setup, SQL migrations (`migrations/`), ERD table structs, and domain repositories. Must not depend on other Gaggle crates. |
| `crates/gaggle-core` | Core agent logic: 7-step `PermissionEngine`, durable run recovery (`RunEngine`), context compaction thresholds, and Code Mode sandbox configuration. |
| `crates/gaggle-tools` | Bundled built-in extensions (`gaggle.fs`, `gaggle.shell`, `gaggle.memory`, `gaggle.web`, `gaggle.browser`) and unified diff generation (`similar`). |
| `crates/gaggle-mcp` | Model Context Protocol (MCP) client integration, extension registry, and active tool count guard (`<= 25` tools per chat). |
| `crates/gaggle-recipes` | Recipe YAML/JSON parser, required vs. optional parameter validation, `minijinja` template rendering, and `croner` schedule evaluation. |
| `crates/gaggle-server` | Axum HTTP/SSE server, REST API handlers, chat JSON export/import, and Web UI serving. |
| `crates/gaggle-cli` | The `gaggle` command-line binary (`serve`, `project`, `export`, `import`, `backup`, `restore`, `db-path`, etc.) and data portability library. |
| `crates/gaggle-ui` | Web UI assets and HTML shell served by `gaggle-server`. |

---

## 4. SQLite Schema & Migration Rules

All schema changes under `crates/gaggle-db/migrations/` **must** follow the conventions defined in [`docs/ERD.md`](./docs/ERD.md):

1. **Primary Keys**: Use time-ordered UUIDv7 stored as `TEXT` (`gaggle_db::connection::new_id()`).
2. **Timestamps**: Use Unix epoch milliseconds stored as `INTEGER` (`gaggle_db::connection::now_ms()`).
3. **JSON Columns**: Store as `TEXT` and enforce `CHECK(json_valid(column_name))` (or `CHECK(column_name IS NULL OR json_valid(column_name))` when nullable).
4. **Boolean Columns**: Store as `INTEGER` (`0` or `1`) with `CHECK(column_name IN (0, 1))`.
5. **Enum Columns**: Store as `TEXT` with `CHECK(column_name IN (...))` matching the allowed values in the ERD.
6. **Secrets & Credentials**: Never add columns that store plaintext API keys or OAuth tokens; always use a `secret_ref` column pointing to the OS Keychain.
7. **New Migrations**: Add sequential migration files (e.g., `0002_description.sql`) and ensure `sqlx::migrate!("./migrations")` runs cleanly from scratch.

---

## 5. Rust Coding Standards

- **Formatting**: Format all code with `cargo fmt --all`.
- **Linting**: Ensure `cargo clippy --workspace --all-targets` produces no warnings.
- **Error Handling**:
  - Use `thiserror` for structured domain errors and `anyhow::Result` with `.context(...)` at application/repository boundaries.
  - Avoid `.unwrap()` or `.expect(...)` in production paths within `gaggle-server` and `gaggle-core` (unless backed by a proven static invariant or inside tests).
- **Structured Logging**: Use `tracing` macros (`info!`, `warn!`, `error!`, `debug!`) with structured key-value fields (e.g., `info!(run_id = %run.id, "...")`) rather than `println!` in library crates.

---

## 6. Testing Guidelines

Every new feature or bug fix should include automated tests:

- For database, repository, and permission tests in `gaggle-db` and `gaggle-core`, use `gaggle_db::connect_in_memory().await?` to provision an isolated in-memory SQLite instance with all migrations and FTS5 triggers applied.
- CLI behavior (chat export/import, memory export, backup/restore) is covered by integration tests in `crates/gaggle-cli/tests/` that run the real binary against temporary directories. Never rewrite or restore over developer data in tests; always use `tempfile::TempDir`.
- See [Data Portability](./docs/development/data-portability.md) for the backup manifest format, restore safety guarantees, and the expected export layout.
- Run the full verification checks before submitting a pull request:
  ```bash
  cargo fmt --all -- --check
  cargo clippy --workspace --all-targets --locked -- -D warnings
  cargo check --workspace --all-targets --locked
  cargo test --workspace --locked
  .github/scripts/check-ui-embedded.sh
  bash -n .github/scripts/*.sh
  ```
- CI and the native package builds run on Ubuntu and macOS for pulls targeting
  `development` or `main`. Pull request builds upload workflow artifacts and do
  not create a release. The `Gaggle.app` and Linux package builds are described
  in [Building and Packaging Gaggle](./docs/development/packaging.md).
- Do not add a workflow that publishes a stable release from a branch push or
  pull request. Stable releases are tag-driven and validated against `main`.

---

## 7. Pull Request Workflow & Commit Conventions

1. Create a descriptive feature or bugfix branch from `development` (e.g., `feat/worktree-manager` or `fix/permission-deny-rule`). Open the pull request against `development`; maintainers promote `development` to `main` after integration.
2. Follow **Conventional Commits** for commit messages:
   - `feat(core): add smart permission LLM risk cache lookup`
   - `fix(db): enforce json_valid check on project_settings.env`
   - `docs(readme): update CLI usage examples`
   - `test(db): add FTS5 trigger update test`
3. Ensure all tests pass and provide a clear summary of your changes and verification steps in the pull request description.

---

## 8. Contribution License

In accordance with Section 5 of the [Apache License, Version 2.0](./LICENSE), any contribution intentionally submitted for inclusion in **Gaggle** shall be under the terms and conditions of the **Apache License, Version 2.0**, without any additional terms or conditions.
