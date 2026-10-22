---
name: Gaggle Architecture
description: ALWAYS load this skill when the user prompt mentions agent loop, durable runs, run recovery, SSE events, checkpoints, permission engine, command rules, tool approvals, Code Mode, compaction, built-in tools, diffs, worktrees, terminal PTY, browser sandbox, MCP extensions, recipes, subagents, cron scheduler, Axum API endpoints, CLI commands, or Web UI in Gaggle.
---

# Skill: Gaggle Core Architecture & Workflows

Follow these guidelines when working in `crates/gaggle-core`, `crates/gaggle-tools`, `crates/gaggle-mcp`, `crates/gaggle-recipes`, `crates/gaggle-server`, `crates/gaggle-cli`, or `crates/gaggle-ui`.

## 1. 7-Step Permission Decision Order (`PermissionEngine`)
Before any `tool_calls` record is executed, the evaluation order in `crates/gaggle-core/src/permission_engine.rs` must be strictly maintained:
1. `chat_only`: Reject all tools unconditionally.
2. `command_rules` (shell commands): Any rule with `action = 'deny'` **always rejects**, even when the chat is in `full` mode.
3. `full`: Execute the tool immediately.
4. `tool_permissions`: `never_allow` rejects, `always_allow` executes, `ask_before` requests approval.
5. Read-Only Annotation (`extension_tools.is_read_only == 1`): Execute without approval.
6. `smart` Mode Only: Check `tool_risk_cache` by `(tool_name, args_fingerprint)`. If cached as read-only, execute without approval.
7. Default: Create a `pending` row in `approvals` and transition `tool_calls.status` to `awaiting_approval`.

## 2. Durable Runs & Restart Recovery (`RunEngine`)
- Every event during a run must be recorded sequentially in `run_events` (`UNIQUE(run_id, seq)`) as the single source of truth for SSE replay.
- On server startup (`gaggle_server::initialize_server_state`), all `runs` in `running` or `waiting_approval` state are recovered from their latest `run_checkpoints` entry and `resume_count` is incremented by 1.

## 3. Unified Extensions & Active Tool Limit
- All built-in capabilities (`gaggle.fs`, `gaggle.shell`, `gaggle.memory`, `gaggle.web`, `gaggle.browser`) are registered in `extensions` with `kind = 'builtin'` and `source = 'bundled'`.
- Keep active tools per chat at or below `RECOMMENDED_MAX_ACTIVE_TOOLS` (25 tools) in `gaggle-mcp`.

## 4. Recipes & Subagents
- Recipe parameters with `required = 1` must not have a `default_value`; parameters with `required = 0` must define a `default_value`.
- Sub-recipes and subagents execute as child chats (`chats.parent_chat_id` and `chats.parent_tool_call_id` with `chat_type = 'subagent'`).
