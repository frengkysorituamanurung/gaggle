use anyhow::Result;
use gaggle_db::repositories::extensions::ExtensionRepository;
use sqlx::SqlitePool;

pub mod fs_tools;
pub mod git_tools;

pub use fs_tools::{
    DEFAULT_MAX_LIST_ENTRIES, DEFAULT_MAX_READ_BYTES, DEFAULT_MAX_SEARCH_MATCHES, FsEntry,
    FsReadOutput, FsSearchMatch, FsToolError, fs_list, fs_read, fs_search,
};
pub use git_tools::{
    DEFAULT_MAX_DIFF_BYTES, GitDiffOutput, GitFileStatus, GitToolError, GitWorkspaceStatus,
    git_diff, git_status,
};

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
        schema_json: r#"{"type":"object","properties":{"query":{"type":"string"}},"required":["query"]}"#,
    },
    BuiltinToolSpec {
        extension_name: "gaggle.memory",
        extension_desc: "Built-in project and global memory tools for Gaggle",
        tool_name: "memory.write",
        description: "Store a fact, preference, decision, todo, or summary in memory",
        is_read_only: false,
        schema_json: r#"{"type":"object","properties":{"kind":{"type":"string"},"title":{"type":"string"},"content":{"type":"string"}},"required":["kind","title","content"]}"#,
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
