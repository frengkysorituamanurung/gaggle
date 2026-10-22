use gaggle_core::{WorkspacePathError, WorkspacePathPolicy};
use serde::{Deserialize, Serialize};
use std::process::Command;
use thiserror::Error;

/// Default maximum unified diff bytes returned by `git_diff` (256 KiB).
pub const DEFAULT_MAX_DIFF_BYTES: usize = 256 * 1024;

#[derive(Debug, Error)]
pub enum GitToolError {
    #[error(transparent)]
    Policy(#[from] WorkspacePathError),

    #[error("Failed to execute git command: {0}")]
    ExecutionFailed(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GitFileStatus {
    pub path: String,
    pub index_status: String,
    pub worktree_status: String,
    pub untracked: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GitWorkspaceStatus {
    pub is_git_repo: bool,
    pub branch: Option<String>,
    pub is_clean: bool,
    pub changed_files: Vec<GitFileStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GitDiffOutput {
    pub is_git_repo: bool,
    pub patch: String,
    pub truncated: bool,
}

/// Inspects a workspace's Git status in read-only mode using `git status --porcelain=v1 -b`.
/// Never modifies the repository or working tree.
pub fn git_status(policy: &WorkspacePathPolicy) -> Result<GitWorkspaceStatus, GitToolError> {
    let root = policy.canonical_root();

    let rev_parse = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["rev-parse", "--is-inside-work-tree"])
        .output()
        .map_err(|e| GitToolError::ExecutionFailed(e.to_string()))?;

    if !rev_parse.status.success() {
        return Ok(GitWorkspaceStatus {
            is_git_repo: false,
            branch: None,
            is_clean: true,
            changed_files: Vec::new(),
        });
    }

    let status_out = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["status", "--porcelain=v1", "-b", "-uall"])
        .output()
        .map_err(|e| GitToolError::ExecutionFailed(e.to_string()))?;

    if !status_out.status.success() {
        return Err(GitToolError::ExecutionFailed(
            String::from_utf8_lossy(&status_out.stderr).into_owned(),
        ));
    }

    let stdout = String::from_utf8_lossy(&status_out.stdout);
    let mut branch = None;
    let mut changed_files = Vec::new();

    for line in stdout.lines() {
        if let Some(branch_header) = line.strip_prefix("## ") {
            let parsed_branch = branch_header
                .split("...")
                .next()
                .unwrap_or(branch_header)
                .trim();
            if !parsed_branch.is_empty() {
                branch = Some(parsed_branch.to_string());
            }
            continue;
        }

        if line.len() < 4 {
            continue;
        }

        let xy = &line[0..2];
        let raw_path = line[3..].trim();
        let index_char = xy.chars().next().unwrap_or(' ');
        let wt_char = xy.chars().nth(1).unwrap_or(' ');
        let untracked = xy == "??";

        changed_files.push(GitFileStatus {
            path: raw_path.to_string(),
            index_status: index_char.to_string(),
            worktree_status: wt_char.to_string(),
            untracked,
        });
    }

    let is_clean = changed_files.is_empty();

    Ok(GitWorkspaceStatus {
        is_git_repo: true,
        branch,
        is_clean,
        changed_files,
    })
}

/// Generates a bounded unified diff for the workspace (or a validated relative file path inside it).
pub fn git_diff(
    policy: &WorkspacePathPolicy,
    relative_file: Option<&str>,
    max_bytes: usize,
) -> Result<GitDiffOutput, GitToolError> {
    let root = policy.canonical_root();

    let status = git_status(policy)?;
    if !status.is_git_repo {
        return Ok(GitDiffOutput {
            is_git_repo: false,
            patch: String::new(),
            truncated: false,
        });
    }

    let mut cmd = Command::new("git");
    cmd.arg("-C")
        .arg(root)
        .args(["diff", "--no-ext-diff", "--no-color"]);

    if let Some(file_arg) = relative_file {
        let validated = policy.resolve_existing_for_read(file_arg)?;
        let rel = validated
            .strip_prefix(root)
            .unwrap_or(&validated)
            .to_path_buf();
        cmd.arg("--").arg(rel);
    }

    let diff_out = cmd
        .output()
        .map_err(|e| GitToolError::ExecutionFailed(e.to_string()))?;

    if !diff_out.status.success() {
        return Err(GitToolError::ExecutionFailed(
            String::from_utf8_lossy(&diff_out.stderr).into_owned(),
        ));
    }

    let raw_bytes = diff_out.stdout;
    let truncated = raw_bytes.len() > max_bytes;
    let slice = if truncated {
        &raw_bytes[..max_bytes]
    } else {
        &raw_bytes[..]
    };

    let patch = String::from_utf8_lossy(slice).into_owned();

    Ok(GitDiffOutput {
        is_git_repo: true,
        patch,
        truncated,
    })
}
