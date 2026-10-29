use anyhow::Result;
use async_trait::async_trait;
use gaggle_core::{ToolExecutor, WorkspacePathPolicy, redact_context_secrets};
use gaggle_db::{
    models::Chat,
    repositories::{
        extensions::ExtensionRepository, memories::MemoryRepository, runs::RunRepository,
        worktrees::WorktreeRepository,
    },
};
use sqlx::SqlitePool;

pub mod browser_tools;
pub mod download;
pub mod final_output;
pub mod fs_tools;
pub mod git_tools;
pub mod shell_tools;
pub mod terminal;
pub mod worktree_manager;
pub use browser_tools::{
    BrowserToolError, chromium_available, chromium_executable, close_all_browsers,
    close_all_browsers_with_pool, close_browser, close_browser_session,
    close_browser_session_with_pool, close_browser_with_pool,
};
pub use download::{DownloadError, DownloadManager, DownloadProgress};
pub use fs_tools::{
    DEFAULT_MAX_LIST_ENTRIES, DEFAULT_MAX_READ_BYTES, DEFAULT_MAX_SEARCH_MATCHES, FsEntry,
    FsReadOutput, FsSearchMatch, FsToolError, FsWriteOutput, fs_list, fs_read, fs_search, fs_write,
};
pub use git_tools::{
    CreatedWorktreeSpec, DEFAULT_MAX_DIFF_BYTES, GitDiffOutput, GitFileStatus, GitToolError,
    GitWorkspaceStatus, git_diff, git_status, git_worktree_create, git_worktree_discard,
    git_worktree_merge,
};
pub use shell_tools::{
    DEFAULT_MAX_SHELL_OUTPUT_BYTES, DEFAULT_SHELL_TIMEOUT_MS, ShellExecOutput, ShellToolError,
    shell_exec,
};
pub use terminal::{DEFAULT_SCROLLBACK_BYTES, TerminalPty, TerminalPtyConfig, TerminalPtyError};
pub use worktree_manager::WorktreeManager;

pub struct BuiltinToolSpec {
    pub extension_name: &'static str,
    pub extension_desc: &'static str,
    pub tool_name: &'static str,
    pub description: &'static str,
    pub is_read_only: bool,
    pub schema_json: &'static str,
}

pub const BUILTIN_TOOLS: &[BuiltinToolSpec] = &[
    BuiltinToolSpec {
        extension_name: "gaggle.fs",
        extension_desc: "Built-in filesystem operations for Gaggle",
        tool_name: "fs.list",
        description: "List files and directories inside the project workspace",
        is_read_only: true,
        schema_json: r#"{"type":"object","properties":{"path":{"type":"string"}}}"#,
    },
    BuiltinToolSpec {
        extension_name: "gaggle.fs",
        extension_desc: "Built-in filesystem operations for Gaggle",
        tool_name: "fs.read",
        description: "Read contents of a file inside the project workspace",
        is_read_only: true,
        schema_json: r#"{"type":"object","properties":{"path":{"type":"string"}},"required":["path"]}"#,
    },
    BuiltinToolSpec {
        extension_name: "gaggle.fs",
        extension_desc: "Built-in filesystem operations for Gaggle",
        tool_name: "fs.search",
        description: "Search UTF-8 text files inside the project workspace",
        is_read_only: true,
        schema_json: r#"{"type":"object","properties":{"query":{"type":"string"}},"required":["query"]}"#,
    },
    BuiltinToolSpec {
        extension_name: "gaggle.fs",
        extension_desc: "Built-in filesystem operations for Gaggle",
        tool_name: "fs.write",
        description: "Write or modify a file inside the project workspace",
        is_read_only: false,
        schema_json: r#"{"type":"object","properties":{"path":{"type":"string"},"content":{"type":"string"}},"required":["path","content"]}"#,
    },
    BuiltinToolSpec {
        extension_name: "gaggle.git",
        extension_desc: "Built-in Git inspection tools for Gaggle",
        tool_name: "git.status",
        description: "Inspect repository branch and modified/untracked files",
        is_read_only: true,
        schema_json: r#"{"type":"object","properties":{}}"#,
    },
    BuiltinToolSpec {
        extension_name: "gaggle.git",
        extension_desc: "Built-in Git inspection tools for Gaggle",
        tool_name: "git.diff",
        description: "Generate unified diff of uncommitted workspace changes",
        is_read_only: true,
        schema_json: r#"{"type":"object","properties":{"path":{"type":"string"}}}"#,
    },
    BuiltinToolSpec {
        extension_name: "gaggle.shell",
        extension_desc: "Built-in shell command execution for Gaggle",
        tool_name: "shell.exec",
        description: "Execute a shell command in the project working directory",
        is_read_only: false,
        schema_json: r#"{"type":"object","properties":{"command":{"type":"string"}},"required":["command"]}"#,
    },
    BuiltinToolSpec {
        extension_name: "gaggle.memory",
        extension_desc: "Built-in project and global memory tools for Gaggle",
        tool_name: "memory.read",
        description: "Search project and global memories using FTS5",
        is_read_only: true,
        schema_json: r#"{"type":"object","properties":{"query":{"type":"string"},"limit":{"type":"integer"}},"required":["query"]}"#,
    },
    BuiltinToolSpec {
        extension_name: "gaggle.memory",
        extension_desc: "Built-in project and global memory tools for Gaggle",
        tool_name: "memory.write",
        description: "Store a fact, preference, decision, todo, or summary in memory",
        is_read_only: false,
        schema_json: r#"{"type":"object","properties":{"kind":{"type":"string","enum":["fact","preference","decision","todo","summary"]},"title":{"type":"string"},"content":{"type":"string"},"tags":{"type":"array","items":{"type":"string"}},"pinned":{"type":"boolean"}},"required":["kind","title","content"]}"#,
    },
    BuiltinToolSpec {
        extension_name: "gaggle.web",
        extension_desc: "Built-in web fetch and search tools",
        tool_name: "web.fetch",
        description: "Fetch a web page over HTTP/HTTPS",
        is_read_only: true,
        schema_json: r#"{"type":"object","properties":{"url":{"type":"string"}},"required":["url"]}"#,
    },
    BuiltinToolSpec {
        extension_name: "gaggle.browser",
        extension_desc: "Built-in headless browser sandbox tools",
        tool_name: "browser.navigate",
        description: "Navigate the sandboxed browser session to a URL",
        is_read_only: false,
        schema_json: r#"{"type":"object","properties":{"url":{"type":"string"}},"required":["url"]}"#,
    },
    BuiltinToolSpec {
        extension_name: "gaggle.browser",
        extension_desc: "Built-in headless browser sandbox tools",
        tool_name: "browser.click",
        description: "Click an element by CSS selector in the sandboxed browser session",
        is_read_only: false,
        schema_json: r#"{"type":"object","properties":{"selector":{"type":"string"}},"required":["selector"]}"#,
    },
    BuiltinToolSpec {
        extension_name: "gaggle.browser",
        extension_desc: "Built-in headless browser sandbox tools",
        tool_name: "browser.type",
        description: "Type text into an input element by CSS selector",
        is_read_only: false,
        schema_json: r#"{"type":"object","properties":{"selector":{"type":"string"},"text":{"type":"string"}},"required":["selector","text"]}"#,
    },
    BuiltinToolSpec {
        extension_name: "gaggle.browser",
        extension_desc: "Built-in headless browser sandbox tools",
        tool_name: "browser.scroll",
        description: "Scroll the page by an amount or to an element",
        is_read_only: false,
        schema_json: r#"{"type":"object","properties":{"selector":{"type":"string"},"amount":{"type":"integer"},"x":{"type":"integer"},"y":{"type":"integer"}}}"#,
    },
    BuiltinToolSpec {
        extension_name: "gaggle.browser",
        extension_desc: "Built-in headless browser sandbox tools",
        tool_name: "browser.screenshot",
        description: "Capture a PNG screenshot of the current page",
        is_read_only: false,
        schema_json: r#"{"type":"object","properties":{}}"#,
    },
    BuiltinToolSpec {
        extension_name: "gaggle.browser",
        extension_desc: "Built-in headless browser sandbox tools",
        tool_name: "browser.extract",
        description: "Extract page text or HTML from the current page",
        is_read_only: false,
        schema_json: r#"{"type":"object","properties":{"format":{"type":"string","enum":["text","html"]}}}"#,
    },
    BuiltinToolSpec {
        extension_name: "gaggle.agent",
        extension_desc: "Built-in agent output tools for Gaggle",
        tool_name: "agent.final_output",
        description: "Produce a structured final output summarizing the agent's work",
        is_read_only: true,
        schema_json: r#"{"type":"object","properties":{"summary":{"type":"string"},"changes":{"type":"array","items":{"type":"string"}},"tests_run":{"type":"array","items":{"type":"string"}},"errors":{"type":"array","items":{"type":"string"}}},"required":["summary"]}"#,
    },
];

/// Registers all built-in Gaggle extensions (`kind = 'builtin'`, `source = 'bundled'`)
/// and their tools in the SQLite database so that permissions, audit logs, and Code Mode
/// share a single unified registry.
pub async fn register_builtin_extensions(pool: &SqlitePool) -> Result<()> {
    let repo = ExtensionRepository::new(pool.clone());
    for spec in BUILTIN_TOOLS {
        let ext = repo
            .upsert_builtin(
                spec.extension_name,
                spec.extension_desc,
                env!("CARGO_PKG_VERSION"),
            )
            .await?;
        repo.upsert_tool(
            &ext.id,
            spec.tool_name,
            spec.description,
            spec.schema_json,
            spec.is_read_only,
        )
        .await?;
    }
    Ok(())
}

/// Computes a unified diff patch between `before` and `after` content for `file_changes`.
pub fn compute_unified_diff(path: &str, before: &str, after: &str) -> String {
    similar::TextDiff::from_lines(before, after)
        .unified_diff()
        .header(&format!("a/{path}"), &format!("b/{path}"))
        .to_string()
}

/// Concrete `ToolExecutor` implementation that dispatches approved tool calls to Gaggle's
/// built-in filesystem, Git, and shell tools while enforcing `WorkspacePathPolicy` and
/// persisting `file_changes` for `fs.write`.
pub struct BuiltinToolExecutor {
    pool: SqlitePool,
}

impl BuiltinToolExecutor {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ToolExecutor for BuiltinToolExecutor {
    async fn execute_tool(
        &self,
        run_id: &str,
        chat: &Chat,
        project_root: &str,
        tool_name: &str,
        args_json: &str,
    ) -> Result<String, String> {
        let effective_root = if chat.mode == "worktree" {
            let wt_repo = WorktreeRepository::new(self.pool.clone());
            let wt = wt_repo
                .get_by_chat_id(&chat.id)
                .await
                .map_err(|e| e.to_string())?
                .ok_or_else(|| {
                    format!(
                        "Chat '{}' is in worktree mode but has no worktrees record",
                        chat.id
                    )
                })?;
            if wt.status != "active" {
                return Err(format!(
                    "Worktree '{}' for chat '{}' is not active (status = '{}')",
                    wt.id, chat.id, wt.status
                ));
            }
            wt.path
        } else {
            project_root.to_string()
        };

        let policy = WorkspacePathPolicy::new(&effective_root).map_err(|e| e.to_string())?;
        let args: serde_json::Value =
            serde_json::from_str(args_json).map_err(|e| format!("Invalid tool args JSON: {e}"))?;

        match tool_name {
            "fs.list" => {
                let path = args.get("path").and_then(|v| v.as_str()).map(String::from);
                let entries = tokio::task::spawn_blocking(move || {
                    fs_list(&policy, path.as_deref(), DEFAULT_MAX_LIST_ENTRIES)
                })
                .await
                .map_err(|e| e.to_string())?
                .map_err(|e| e.to_string())?;
                serde_json::to_string(&entries).map_err(|e| e.to_string())
            }
            "fs.read" => {
                let path = args
                    .get("path")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "Missing required 'path' argument".to_string())?
                    .to_string();
                let out = tokio::task::spawn_blocking(move || {
                    fs_read(&policy, path, DEFAULT_MAX_READ_BYTES)
                })
                .await
                .map_err(|e| e.to_string())?
                .map_err(|e| e.to_string())?;
                serde_json::to_string(&out).map_err(|e| e.to_string())
            }
            "fs.search" => {
                let query = args
                    .get("query")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "Missing required 'query' argument".to_string())?
                    .to_string();
                let matches = tokio::task::spawn_blocking(move || {
                    fs_search(&policy, &query, DEFAULT_MAX_SEARCH_MATCHES)
                })
                .await
                .map_err(|e| e.to_string())?
                .map_err(|e| e.to_string())?;
                serde_json::to_string(&matches).map_err(|e| e.to_string())
            }
            "fs.write" => {
                let path = args
                    .get("path")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "Missing required 'path' argument".to_string())?
                    .to_string();
                let content = args
                    .get("content")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "Missing required 'content' argument".to_string())?
                    .to_string();
                let mode = chat.mode.clone();

                let write_out =
                    tokio::task::spawn_blocking(move || fs_write(&policy, &mode, path, &content))
                        .await
                        .map_err(|e| e.to_string())?
                        .map_err(|e| e.to_string())?;

                let run_repo = RunRepository::new(self.pool.clone());
                run_repo
                    .record_file_change(
                        run_id,
                        &chat.id,
                        &write_out.relative_path,
                        &write_out.change_type,
                        write_out.additions,
                        write_out.deletions,
                        &write_out.patch,
                        write_out.before_sha.as_deref(),
                        &write_out.after_sha,
                    )
                    .await
                    .map_err(|e| e.to_string())?;

                serde_json::to_string(&write_out).map_err(|e| e.to_string())
            }
            "git.status" => {
                let status = tokio::task::spawn_blocking(move || git_status(&policy))
                    .await
                    .map_err(|e| e.to_string())?
                    .map_err(|e| e.to_string())?;
                serde_json::to_string(&status).map_err(|e| e.to_string())
            }
            "git.diff" => {
                let path = args.get("path").and_then(|v| v.as_str()).map(String::from);
                let diff = tokio::task::spawn_blocking(move || {
                    git_diff(&policy, path.as_deref(), DEFAULT_MAX_DIFF_BYTES)
                })
                .await
                .map_err(|e| e.to_string())?
                .map_err(|e| e.to_string())?;
                serde_json::to_string(&diff).map_err(|e| e.to_string())
            }
            "shell.exec" => {
                let command = args
                    .get("command")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "Missing required 'command' argument".to_string())?;
                let out = shell_exec(
                    &policy,
                    &chat.mode,
                    None,
                    command,
                    DEFAULT_SHELL_TIMEOUT_MS,
                    DEFAULT_MAX_SHELL_OUTPUT_BYTES,
                )
                .await
                .map_err(|e| e.to_string())?;
                serde_json::to_string(&out).map_err(|e| e.to_string())
            }
            "memory.write" => {
                let kind = args
                    .get("kind")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "Missing required 'kind' argument".to_string())?;
                let title = args
                    .get("title")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "Missing required 'title' argument".to_string())?;
                let content = args
                    .get("content")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "Missing required 'content' argument".to_string())?;
                let tags: Vec<String> = args
                    .get("tags")
                    .and_then(|v| v.as_array())
                    .map(|items| {
                        items
                            .iter()
                            .filter_map(|item| item.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default();
                let pinned = args
                    .get("pinned")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                // Memory content can accidentally include credentials. Never persist secrets in
                // SQLite; the agent receives the redacted form in the tool result.
                let title = redact_context_secrets(title);
                let content = redact_context_secrets(content);
                let tags: Vec<String> = tags
                    .into_iter()
                    .map(|tag| redact_context_secrets(&tag))
                    .collect();

                let repo = MemoryRepository::new(self.pool.clone());
                // Project scope always comes from the authenticated chat, never from model input.
                let memory = repo
                    .create_with_options(
                        Some(&chat.project_id),
                        Some(&chat.id),
                        Some(run_id),
                        kind,
                        &title,
                        &content,
                        &tags,
                        pinned,
                    )
                    .await
                    .map_err(|e| e.to_string())?;
                let stored_tags = repo
                    .list_tags(&memory.id)
                    .await
                    .map_err(|e| e.to_string())?;
                serde_json::to_string(&serde_json::json!({
                    "memory": memory,
                    "tags": stored_tags
                }))
                .map_err(|e| e.to_string())
            }
            "browser.navigate" => {
                let url = args
                    .get("url")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "Missing required 'url' argument".to_string())?;
                browser_tools::browser_navigate(&self.pool, &chat.id, Some(run_id), url)
                    .await
                    .map_err(|e| e.to_string())
            }
            "browser.click" => {
                let selector = args
                    .get("selector")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "Missing required 'selector' argument".to_string())?;
                browser_tools::browser_click(&self.pool, &chat.id, Some(run_id), selector)
                    .await
                    .map_err(|e| e.to_string())
            }
            "browser.type" => {
                let selector = args
                    .get("selector")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "Missing required 'selector' argument".to_string())?;
                let text = args
                    .get("text")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "Missing required 'text' argument".to_string())?;
                browser_tools::browser_type(&self.pool, &chat.id, Some(run_id), selector, text)
                    .await
                    .map_err(|e| e.to_string())
            }
            "browser.scroll" => {
                let selector = args.get("selector").and_then(|v| v.as_str());
                let amount = args.get("amount").and_then(|v| v.as_i64());
                let x = args.get("x").and_then(|v| v.as_i64());
                let y = args.get("y").and_then(|v| v.as_i64());
                browser_tools::browser_scroll(
                    &self.pool,
                    &chat.id,
                    Some(run_id),
                    selector,
                    amount,
                    x,
                    y,
                )
                .await
                .map_err(|e| e.to_string())
            }
            "browser.screenshot" => {
                browser_tools::browser_screenshot(&self.pool, &chat.id, Some(run_id))
                    .await
                    .map_err(|e| e.to_string())
            }
            "browser.extract" => {
                let format = args
                    .get("format")
                    .and_then(|v| v.as_str())
                    .unwrap_or("text");
                browser_tools::browser_extract(&self.pool, &chat.id, Some(run_id), format)
                    .await
                    .map_err(|e| e.to_string())
            }
            "memory.read" => {
                let query = args
                    .get("query")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .trim();
                let limit = args
                    .get("limit")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(10)
                    .clamp(1, 50);
                let repo = MemoryRepository::new(self.pool.clone());
                let memories = if query.is_empty() {
                    repo.list_by_project(&chat.project_id, limit)
                        .await
                        .map_err(|e| e.to_string())?
                } else {
                    repo.search_scoped(Some(&chat.project_id), query, limit)
                        .await
                        .map_err(|e| e.to_string())?
                };

                let mut items = Vec::with_capacity(memories.len());
                for memory in memories {
                    let tags = repo
                        .list_tags(&memory.id)
                        .await
                        .map_err(|e| e.to_string())?;
                    let _ = repo.touch_last_used(&memory.id).await;
                    items.push(serde_json::json!({
                        "memory": memory,
                        "tags": tags
                    }));
                }
                serde_json::to_string(&serde_json::json!({
                    "query": query,
                    "count": items.len(),
                    "results": items
                }))
                .map_err(|e| e.to_string())
            }
            "agent.final_output" => {
                // Produce a structured final output
                let summary = args
                    .get("summary")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Task completed");
                let mut output = crate::final_output::FinalOutput::new(summary);
                if let Some(changes) = args.get("changes").and_then(|v| v.as_array()) {
                    for change in changes {
                        if let Some(s) = change.as_str() {
                            output.add_change(s);
                        }
                    }
                }
                if let Some(tests) = args.get("tests_run").and_then(|v| v.as_array()) {
                    for test in tests {
                        if let Some(s) = test.as_str() {
                            output.add_test(s);
                        }
                    }
                }
                if let Some(errors) = args.get("errors").and_then(|v| v.as_array()) {
                    for error in errors {
                        if let Some(s) = error.as_str() {
                            output.add_error(s);
                        }
                    }
                }
                serde_json::to_string(&output).map_err(|e| e.to_string())
            }
            other => Err(format!("Unsupported built-in tool '{other}'")),
        }
    }
}
