use gaggle_core::{WorkspacePathError, WorkspacePathPolicy};
use serde::{Deserialize, Serialize};
use std::{process::Stdio, time::Duration};
use thiserror::Error;
use tokio::{io::AsyncReadExt, process::Command};

/// Default wall-clock timeout for `shell.exec` (30 seconds).
pub const DEFAULT_SHELL_TIMEOUT_MS: u64 = 30_000;
/// Default maximum bytes captured per stream (`stdout` / `stderr`) (128 KiB).
pub const DEFAULT_MAX_SHELL_OUTPUT_BYTES: usize = 128 * 1024;

/// Allowlisted environment variables inherited from the host process.
/// Any API keys, tokens, or secrets in the host environment are stripped.
const SAFE_ENV_VARS: &[&str] = &["PATH", "HOME", "USER", "LANG", "LC_ALL", "TERM", "TMPDIR"];

#[derive(Debug, Error)]
pub enum ShellToolError {
    #[error(transparent)]
    Policy(#[from] WorkspacePathError),

    #[error("Shell execution is forbidden in readonly workspace mode")]
    ReadOnlyMode,

    #[error("Command must not be empty")]
    EmptyCommand,

    #[error("Command timed out after {timeout_ms} ms")]
    Timeout {
        timeout_ms: u64,
        stdout: String,
        stderr: String,
    },

    #[error("Failed to spawn or await process: {0}")]
    ProcessError(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ShellExecOutput {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub stdout_truncated: bool,
    pub stderr_truncated: bool,
}

/// Executes a shell command inside a validated workspace directory with:
/// - Explicit `readonly` workspace mode rejection
/// - Sanitized environment (`env_clear()` + safe allowlist)
/// - Wall-clock timeout with child kill-on-drop
/// - Bounded `stdout` and `stderr` capture (`max_output_bytes`)
pub async fn shell_exec(
    policy: &WorkspacePathPolicy,
    workspace_mode: &str,
    relative_cwd: Option<&str>,
    command_str: &str,
    timeout_ms: u64,
    max_output_bytes: usize,
) -> Result<ShellExecOutput, ShellToolError> {
    if workspace_mode == "readonly" {
        return Err(ShellToolError::ReadOnlyMode);
    }

    if command_str.trim().is_empty() {
        return Err(ShellToolError::EmptyCommand);
    }

    let cwd = match relative_cwd {
        Some(rel) => policy.resolve_existing_for_read(rel)?,
        None => policy.canonical_root().to_path_buf(),
    };

    if !cwd.is_dir() {
        return Err(ShellToolError::ProcessError(format!(
            "Working directory '{}' is not a directory",
            cwd.display()
        )));
    }

    let mut cmd = Command::new("/bin/sh");
    cmd.arg("-c")
        .arg(command_str)
        .current_dir(&cwd)
        .env_clear()
        .kill_on_drop(true)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    for key in SAFE_ENV_VARS {
        if let Ok(val) = std::env::var(key) {
            cmd.env(key, val);
        }
    }

    let mut child = cmd
        .spawn()
        .map_err(|e| ShellToolError::ProcessError(e.to_string()))?;

    let stdout_pipe = child
        .stdout
        .take()
        .ok_or_else(|| ShellToolError::ProcessError("Failed to capture stdout".to_string()))?;
    let stderr_pipe = child
        .stderr
        .take()
        .ok_or_else(|| ShellToolError::ProcessError("Failed to capture stderr".to_string()))?;

    let stdout_task = tokio::spawn(read_bounded_stream(stdout_pipe, max_output_bytes));
    let stderr_task = tokio::spawn(read_bounded_stream(stderr_pipe, max_output_bytes));

    let wait_res = tokio::time::timeout(Duration::from_millis(timeout_ms), child.wait()).await;

    match wait_res {
        Ok(Ok(status)) => {
            let (stdout, stdout_truncated) = stdout_task
                .await
                .map_err(|e| ShellToolError::ProcessError(e.to_string()))??;
            let (stderr, stderr_truncated) = stderr_task
                .await
                .map_err(|e| ShellToolError::ProcessError(e.to_string()))??;

            Ok(ShellExecOutput {
                exit_code: status.code().unwrap_or(-1),
                stdout,
                stderr,
                stdout_truncated,
                stderr_truncated,
            })
        }
        Ok(Err(e)) => Err(ShellToolError::ProcessError(e.to_string())),
        Err(_) => {
            let _ = child.kill().await;
            let (stdout, _) = stdout_task
                .await
                .unwrap_or(Ok((String::new(), false)))
                .unwrap_or_default();
            let (stderr, _) = stderr_task
                .await
                .unwrap_or(Ok((String::new(), false)))
                .unwrap_or_default();
            Err(ShellToolError::Timeout {
                timeout_ms,
                stdout,
                stderr,
            })
        }
    }
}

async fn read_bounded_stream<R: tokio::io::AsyncRead + Unpin>(
    mut reader: R,
    max_bytes: usize,
) -> Result<(String, bool), ShellToolError> {
    let mut collected = Vec::with_capacity(max_bytes.min(4096));
    let mut truncated = false;
    let mut buf = [0u8; 4096];

    loop {
        let n = reader
            .read(&mut buf)
            .await
            .map_err(|e| ShellToolError::ProcessError(e.to_string()))?;
        if n == 0 {
            break;
        }

        if collected.len() < max_bytes {
            let remaining = max_bytes - collected.len();
            if n <= remaining {
                collected.extend_from_slice(&buf[..n]);
            } else {
                collected.extend_from_slice(&buf[..remaining]);
                truncated = true;
            }
        } else {
            truncated = true;
        }
    }

    Ok((String::from_utf8_lossy(&collected).into_owned(), truncated))
}
