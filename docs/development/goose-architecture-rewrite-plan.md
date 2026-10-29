# Goose Architecture Rewrite Plan

This is the implementation plan for aligning Gaggle with the local Goose
reference checkout (`goose/`, Apache-2.0, snapshot `775b3958c`). The checkout is
read-only. This plan targets Goose's architecture and user-facing capabilities;
it does not replace Gaggle by copying another workspace over it.

The governing decision is [ADR 0004](../adr/0004-goose-architecture-alignment.md).
The older [Goose-to-Gaggle migration plan](goose-to-gaggle-migration-plan.md)
describes incremental behavior ports and is superseded for new work.

## Current state observed

| Area | Gaggle | Goose reference | Rewrite direction |
|---|---|---|---|
| Agent runtime | `gaggle-core::AgentRuntime`, a bounded turn loop; inference and tool/approval handling are now extracted operation boundaries | `goose-agent` plus ordered operations/inference and `StateMachineServices` | Continue replacing the loop with explicit state transitions and effect application; persist each effect through Gaggle repositories |
| Persistence | SQLx SQLite migrations; projects, chats, runs, ordered run events, checkpoints, approvals, tools, recipes, and memories | Goose session manager and session/message persistence | Preserve Gaggle as source of truth behind a runtime/session adapter; add versioned migrations only |
| Providers | Gaggle provider trait and adapters; model catalog discovery now supports native OpenAI-compatible, Google, Anthropic, and Ollama endpoints | `goose-provider-types`, `goose-providers`, provider inventory | Continue aligning adapter capabilities and add provider-specific streaming/model metadata; retain secret references and normalize errors/streams |
| Extensions | `gaggle-mcp` and `gaggle-tools`, with central permission checks | `goose-mcp`, extension manager, built-ins and platform extensions | Port compatible extension lifecycle and tools without bypassing Gaggle's permission engine |
| Context | Context assembler and basic persisted compaction | Context-management crate, structured summaries, tool-pair compaction | Port context operations after runtime effects are durable and tested |
| UI | React/Vite app served by Axum; Tauri desktop shell now binds an available loopback port and shuts down its server on exit | React/Electron desktop app | Adopt desktop workflows in stages; keep UI calls on a stable backend contract until cutover |
| Delivery | CLI/server and macOS/Linux packaging | CLI, desktop, API/ACP, and Windows packaging | Add entry points/platforms only with parity and release tests |

## Phases

### 0. Baseline and compatibility contract

- Record this Goose snapshot and the starting Gaggle workspace/API/schema.
- Inventory user-visible Goose workflows, providers, built-ins, MCP transports,
  settings, session export/import, desktop behavior, CLI, ACP, and supported OSes.
- Capture baseline Gaggle tests and representative database fixtures.
- Define compatibility and deprecation rules for Gaggle CLI/API and its database.

**Exit:** every Goose capability has an owner, target layer, dependency, and
observable acceptance test; the Gaggle data-preservation contract is tested.

### 1. Runtime and durable state machine

- Introduce Goose-style discrete operations and inference steps in the runtime.
- **Started:** `InferenceOperation` owns provider completion/stream collection;
  streamed deltas go through durable-event persistence and provider,
  persistence, and cancellation outcomes remain distinct.
- **Started:** `ToolCallingOperation` owns tool lookup, the permission decision,
  approval persistence, and approved execution while preserving the existing
  server-side `PermissionEngine`.
- Make effects explicit; persist message/tool/run changes and ordered events
  before advancing to the next operation.
- Port steering, cancellation, retry/empty-response handling, max-turn control,
  approval/resume, hooks, and tool-pair compaction in independently testable
  operations.
- Keep uncertain tool outcomes distinct during recovery; never replay them
  blindly.

**Exit:** integration tests cover completion, multiple tool calls, denial,
approval/resume, steering, cancellation during inference and tools, crash/restart,
and event replay without duplicate side effects.

### 2. Providers and context management

- Align provider capability types and streaming normalization with Goose.
- **Started:** provider catalog discovery now reads provider-native model lists
  without persisting guessed context/tool capabilities; the Models screen lets
  users select a discovered key for manual configuration.
- Port provider families incrementally, beginning with the providers in Goose's
  default distribution; keep each adapter independently testable.
- Add structured context summaries, token-limit heuristics, tool-pair
  compaction, and continuation behavior.
- Preserve redaction and keep credential values out of SQLite, prompts, and logs.

**Exit:** provider mock-server tests cover success, malformed output, streaming,
timeouts, cancellation, rate limits, and safe retry policy; compaction tests
preserve unresolved tool calls, approvals, and system/project instructions.

### 3. Extensions, tools, and agent capabilities

- Align MCP connection lifecycle, tool schema normalization, progress, and
  cancellation.
- Port compatible Goose built-ins, skills, hooks, slash commands, recipes,
  subagents, scheduler, and local inference behind explicit Gaggle interfaces.
- Keep filesystem roots canonicalized and all tool execution behind
  `PermissionEngine`; enforce tool, recursion, output, and time budgets.

**Exit:** each side-effecting capability has permission, timeout, output-limit,
cancellation, audit-event, and adversarial-input tests.

### 4. Desktop and external interfaces

- Replace the browser-first experience with a Goose-style native desktop
  workflow, including onboarding, settings, chat, extensions, recipes,
  schedules, diagnostics, history, and approvals.
- **Started:** the Tauri shell no longer assumes port `4321` or sleeps before
  navigation; it binds loopback port `0`, navigates to the bound server, and
  signals graceful Axum shutdown on app exit.
- **Started:** Electron desktop shell rebranded from Goose to Gaggle (binary
  name, package name, protocol scheme, branding). ACP backend dependencies
  added to gaggle-server. Minimal ACP-compatible endpoints (/health, /status)
  and in-memory session manager implemented.
- **Critical assessment:** Full ACP server implementation requires porting
  Goose's `HandleDispatchFrom<Client>` trait (4000+ lines) which wraps
  `StateMachineServices`. This is not a simple adapter - it requires
  implementing the entire ACP JSON-RPC protocol including initialize,
  new_session, prompt, cancel, session management, config options, and
  streaming notifications. The recommended approach is to either:
  1. Port Goose's ACP server module wholesale and adapt it to Gaggle's
     data model, or
  2. Create a compatibility bridge that translates ACP requests to
     Gaggle's REST API.
  Option 1 is more robust but requires significant effort. Option 2 is
  faster but may have limitations.
- Align CLI and ACP/API behaviors needed for Goose parity while retaining a
  stable compatibility layer during UI cutover.
- Decide Electron/Tauri and Windows packaging from measured capability and
  maintenance requirements before moving packaging code.

**Exit:** end-to-end tests cover first run, provider configuration, a chat with
tool approval, cancellation/restart, extension setup, and data export/import on
each supported platform.

### 5. Data transition and release

- Keep Gaggle's database format until an explicit migration is proven necessary.
- **Started:** `gaggle import goose-session --source-db ... --session ...
  --project ...` imports one Goose session from a read-only SQLite connection
  through Gaggle's validated chat-bundle transaction. Imported chats default to
  `ask`; historical usage and opaque session/message metadata are stored in
  dedicated, exported artifact tables with sensitive metadata values redacted.
  Provider credentials are not copied; provider configuration remains inactive
  import metadata and must be configured again in Gaggle.
- If schema or storage format changes, add a versioned migration, backup/restore
  tests, upgrade tests from supported releases, and recovery instructions.
- Retain session/chat export and document credential reconfiguration; never
  export secret values.
- Remove legacy UI/runtime paths only after usage cutover and migration checks.

**Exit:** upgrade and rollback/recovery procedures are documented and tested;
no existing Gaggle data is silently discarded; CI, packaging, and smoke tests
pass on every supported OS.

## Operating rules

- Goose is a reference, not a runtime dependency or writable worktree.
- No phase may weaken Gaggle's `command_rules` deny behavior, approval
  verification, canonical project-root checks, secret handling, or durable run
  event ordering.
- One phase should be implemented and verified at a time; do not combine a
  storage migration with a runtime or UI cutover.
- Before removing a Gaggle feature, map it to a Goose equivalent or record it as
  an intentional product difference with an acceptance test.
- Require formatting, clippy, workspace, and phase-specific test gates in CI
  before considering a phase complete. Do not run local builds or tests when
  the user has prohibited them.
