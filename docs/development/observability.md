# Observability, Diagnostics & Error Reporting

Gaggle keeps observability **local and explicit**. There is no automatic telemetry, crash
reporting, remote log aggregation, or diagnostics upload. Logs are written to the process stderr,
and a diagnostics bundle is only produced when a user runs `gaggle doctor` or clicks an export
control in Settings.

## Structured logs

Logs use `tracing` with structured fields. Runtime work is wrapped in a `gaggle.operation` span that
carries the correlation identifiers:

| Field | Meaning | Unknown value |
| :--- | :--- | :--- |
| `chat_id` | Active chat | `-` |
| `run_id` | Durable run | `-` |
| `tool_call_id` | Current tool call | `-` |

The identifiers are propagated through `AgentRuntime`, `RunRepository`, and `PermissionEngine`.
Repository log entries are metadata-only (run status, event kind/sequence, tool name, argument
byte count, status, duration); `run_events` payloads are never logged.

Default verbosity is `info`. Set `RUST_LOG` to change it:

```bash
RUST_LOG=debug cargo run -p gaggle-cli -- serve
```

The output format is plain text (no ANSI escape sequences) so macOS and Linux logs are comparable.
Initialization is additive and returns an error instead of panicking when a subscriber is already
installed, which keeps tests and embedders free to provide their own subscriber.

## Redaction

Redaction is applied **at the tracing writer boundary**, not only at individual call sites. Every
completed log line passes through `gaggle_core::observability::redact_log_text` before it reaches
stderr. The pipeline:

1. Redacts `-----BEGIN ... PRIVATE KEY-----` blocks.
2. Redacts `key: value` / `key=value` credential assignments.
3. Redacts `Bearer <token>` and recognized token shapes (`sk-`, `ghp_`, `github_pat_`, `xox?-`,
   `AKIA...`).
4. Redacts JSON-style values whose key is sensitive (`api_key`, `token`, `password`,
   `authorization`, `secret`, `private_key`, `command`, `prompt`, `content`, `patch`, `diff`,
   `source`, `stdout`, `stderr`, `output`, `messages`, `args`, and related names).

Tool arguments are summarized before logging: only a SHA-256 `args_fingerprint`, the argument key
names, byte length, and truncated non-sensitive scalar values are recorded. Prompts, source files,
raw command output, environment dumps, and full tool results are never written to the default log
stream.

## `gaggle doctor`

```bash
gaggle doctor           # human-readable summary
gaggle doctor --json    # full machine-readable report
```

The report contains:

- **Build identity** — service, version, Git commit, Rust compiler version, build profile, and
  target. Packagers can inject `GAGGLE_GIT_COMMIT` and `GAGGLE_RUSTC_VERSION` at build time; local
  builds report `unknown` for values that were not injected.
- **Core health** — database reachability, applied migrations, `foreign_keys`, journal mode,
  required tables, and pending approvals.
- **Optional capabilities** — every optional capability carries the reason it is unavailable,
  needs setup, or is unverified, plus an action when one is available. Diagnostics never probes the
  network, so Ollama is reported as `unverified` rather than guessed.

`gaggle doctor` exits non-zero when a core check fails. Fix the reported action and rerun the
command.

## Redacted diagnostics export

Two API surfaces exist:

- `GET /api/diagnostics` — live report (build, health, capabilities, counts).
- `GET /api/diagnostics/export` — explicit export with the same report plus up to ten recent failed
  or paused runs. This is the only endpoint that includes recent failure summaries, and it is
  never called automatically.

The export includes a `privacy` block stating that telemetry is disabled and that prompts, source
files, raw tool output, and secrets are excluded. Failure messages are redacted and length-bounded,
and a failure category is extracted from the actionable message when present.

In the Web UI, open **Settings → Diagnostics** to refresh, copy, or download a redacted export.
Failed chat turns render a problem notice with the failure category, retry guidance, and a
**Copy redacted diagnostics** action. Database and workspace paths can reveal local directory
names, so review a bundle before sharing it.

## Actionable errors

Run failures use `ActionableError`, which reports:

- failure category (`provider_timeout`, `rate_limited`, `authentication`, `permission_denied`,
  `database`, `budget_exhausted`, and so on),
- affected operation (`agent.turn`, `chat.load`, `diagnostics.export`, ...),
- data preservation status (`preserved`, `not_preserved`, `unknown`),
- whether a retry is safe, and concrete retry guidance.

The repository stores the same text on `runs.error`, and the `run.failed` event payload includes
`failure_category`, `operation`, `data_preservation`, `retryable`, and `retry_guidance` alongside
the backward-compatible `error` string.

## Tests

Focused coverage lives in:

- `crates/gaggle-core/src/observability.rs` — redaction primitives and the writer boundary.
- `crates/gaggle-core/src/errors.rs` — failure mapping and actionable messages.
- `crates/gaggle-core/tests/issue23_observability_test.rs` — correlation propagation, redacted
  logs, diagnostics export, and provider failure events.
- `crates/gaggle-server/tests/issue23_observability_api_test.rs` — diagnostics and export APIs.
- `crates/gaggle-cli/tests/doctor_test.rs` — `gaggle doctor` human and JSON output.
- `crates/gaggle-ui/src/lib.rs` — Settings export controls and chat problem notices.

## Non-goals

- No automatic telemetry or crash reporting.
- No remote log aggregation.
- No prompt, source-file, or raw tool-output capture in default logs.
