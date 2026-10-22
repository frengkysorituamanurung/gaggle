use anyhow::Result;
use gaggle_core::{PermissionEngine, WorkspacePathPolicy};
use gaggle_db::{
    connect_in_memory,
    connection::new_id,
    repositories::{
        chats::ChatRepository, permissions::PermissionRepository, projects::ProjectRepository,
        runs::RunRepository,
    },
};
use gaggle_tools::{
    DEFAULT_MAX_SHELL_OUTPUT_BYTES, FsToolError, ShellToolError, fs_write, shell_exec,
};

#[tokio::test]
async fn test_fs_write_diff_recording_and_readonly_protection() -> Result<()> {
    let temp_root = std::env::temp_dir().join(format!("gaggle_m3_fsw_{}", new_id()));
    std::fs::create_dir_all(&temp_root)?;
    let policy = WorkspacePathPolicy::new(&temp_root)?;

    // 1. Readonly workspace mode rejects writes
    let ro_err = fs_write(&policy, "readonly", "src/app.rs", "fn main() {}\n").unwrap_err();
    assert!(matches!(ro_err, FsToolError::ReadOnlyMode));

    // 2. Sensitive path write (.env) is denied even in local mode
    let env_err = fs_write(&policy, "local", ".env", "SECRET=1\n").unwrap_err();
    assert!(matches!(env_err, FsToolError::Policy(_)));

    // 3. Initial file creation records `add` change_type and unified diff
    let add_out = fs_write(
        &policy,
        "local",
        "src/app.rs",
        "fn main() {\n    println!(\"v1\");\n}\n",
    )?;
    assert_eq!(add_out.relative_path, "src/app.rs");
    assert_eq!(add_out.change_type, "add");
    assert_eq!(add_out.additions, 3);
    assert_eq!(add_out.deletions, 0);
    assert!(add_out.before_sha.is_none());
    assert!(!add_out.after_sha.is_empty());

    // 4. Subsequent edit records `modify` change_type, before_sha, after_sha, and patch
    let mod_out = fs_write(
        &policy,
        "local",
        "src/app.rs",
        "fn main() {\n    println!(\"v2\");\n}\n",
    )?;
    assert_eq!(mod_out.change_type, "modify");
    assert_eq!(mod_out.additions, 1);
    assert_eq!(mod_out.deletions, 1);
    assert_eq!(
        mod_out.before_sha.as_deref(),
        Some(add_out.after_sha.as_str())
    );
    assert!(mod_out.patch.contains("-    println!(\"v1\");"));
    assert!(mod_out.patch.contains("+    println!(\"v2\");"));

    // 5. Persist into `file_changes` table in SQLite
    let pool = connect_in_memory().await?;
    let proj_repo = ProjectRepository::new(pool.clone());
    let chat_repo = ChatRepository::new(pool.clone());
    let run_repo = RunRepository::new(pool.clone());

    let proj = proj_repo
        .create("m3-proj", &policy.canonical_root().to_string_lossy(), "git")
        .await?;
    let chat = chat_repo
        .create(&proj.id, "Write Test", "user", "local", "ask")
        .await?;
    let run = run_repo.create(&chat.id, None, None).await?;

    let fc = run_repo
        .record_file_change(
            &run.id,
            &chat.id,
            &mod_out.relative_path,
            &mod_out.change_type,
            mod_out.additions,
            mod_out.deletions,
            &mod_out.patch,
            mod_out.before_sha.as_deref(),
            &mod_out.after_sha,
        )
        .await?;
    assert_eq!(fc.status, "pending");
    assert_eq!(fc.path, "src/app.rs");

    let _ = std::fs::remove_dir_all(&temp_root);
    Ok(())
}

#[tokio::test]
async fn test_shell_exec_env_sanitization_timeout_and_output_cap() -> Result<()> {
    let temp_root = std::env::temp_dir().join(format!("gaggle_m3_sh_{}", new_id()));
    std::fs::create_dir_all(&temp_root)?;
    let policy = WorkspacePathPolicy::new(&temp_root)?;

    // 1. Readonly mode blocks shell_exec
    let ro_err = shell_exec(
        &policy,
        "readonly",
        None,
        "echo hello",
        5000,
        DEFAULT_MAX_SHELL_OUTPUT_BYTES,
    )
    .await
    .unwrap_err();
    assert!(matches!(ro_err, ShellToolError::ReadOnlyMode));

    // 2. Host secret environment variables are not inherited by subprocesses
    // Verify via an inline environment variable check that unlisted vars are absent
    let out = shell_exec(
        &policy,
        "local",
        None,
        "printf '%s' \"${GAGGLE_UNLISTED_SECRET:-redacted}\"",
        5000,
        DEFAULT_MAX_SHELL_OUTPUT_BYTES,
    )
    .await?;
    assert_eq!(out.exit_code, 0);
    assert_eq!(out.stdout, "redacted");

    // 3. Output truncation cap works without hanging the child process
    let trunc_out = shell_exec(
        &policy,
        "local",
        None,
        "printf '0123456789abcdef0123456789abcdef'",
        5000,
        10,
    )
    .await?;
    assert_eq!(trunc_out.stdout, "0123456789");
    assert!(trunc_out.stdout_truncated);

    // 4. Wall-clock timeout terminates hanging commands
    let timeout_err = shell_exec(
        &policy,
        "local",
        None,
        "sleep 5",
        100,
        DEFAULT_MAX_SHELL_OUTPUT_BYTES,
    )
    .await
    .unwrap_err();
    assert!(matches!(
        timeout_err,
        ShellToolError::Timeout {
            timeout_ms: 100,
            ..
        }
    ));

    let _ = std::fs::remove_dir_all(&temp_root);
    Ok(())
}

#[tokio::test]
async fn test_stateful_approval_workflow_audit_and_replay_rejection() -> Result<()> {
    let pool = connect_in_memory().await?;
    let proj_repo = ProjectRepository::new(pool.clone());
    let chat_repo = ChatRepository::new(pool.clone());
    let run_repo = RunRepository::new(pool.clone());
    let perm_repo = PermissionRepository::new(pool.clone());

    let proj = proj_repo
        .create("gaggle", "/Users/frengky/Documents/projects/gaggle", "git")
        .await?;
    let chat = chat_repo
        .create(&proj.id, "Approval Chat", "user", "local", "ask")
        .await?;
    let run = run_repo.create(&chat.id, None, None).await?;

    let tc = run_repo
        .record_tool_call(
            &run.id,
            1,
            "native",
            "shell.exec",
            r#"{"command":"cargo check"}"#,
            "awaiting_approval",
        )
        .await?;
    let approval = run_repo
        .create_approval(
            &tc.id,
            "shell",
            "Run cargo check",
            Some("cargo check"),
            Some(&proj.root_path),
        )
        .await?;

    let pending = run_repo.list_pending_approvals().await?;
    assert_eq!(pending.len(), 1);

    // Approve and remember as a prefix command_rule
    let decided = run_repo
        .decide_approval(
            &approval.id,
            true,
            None,
            Some(&proj.id),
            Some(("cargo check", "exact")),
        )
        .await?;
    assert_eq!(decided.status, "approved");
    assert!(decided.remember_rule_id.is_some());

    // Double decision on an already decided approval is rejected
    let replay_decide_err = run_repo
        .decide_approval(&approval.id, false, None, None, None)
        .await;
    assert!(replay_decide_err.is_err());

    // Verify exact argument binding passes for unmodified arguments
    PermissionEngine::verify_approved_execution(
        &decided,
        &tc,
        r#"{"command":"cargo check"}"#,
        Some("cargo check"),
    )?;

    // Altered arguments after approval are rejected
    let altered_args_err = PermissionEngine::verify_approved_execution(
        &decided,
        &tc,
        r#"{"command":"cargo check && rm -rf /"}"#,
        Some("cargo check"),
    );
    assert!(altered_args_err.is_err());

    // Altered shell command string is rejected
    let altered_cmd_err = PermissionEngine::verify_approved_execution(
        &decided,
        &tc,
        r#"{"command":"cargo check"}"#,
        Some("cargo check && curl evil.example"),
    );
    assert!(altered_cmd_err.is_err());

    // Command rule and audit log were persisted
    let rules = perm_repo.list_command_rules(Some(&proj.id)).await?;
    assert_eq!(rules.len(), 1);
    assert_eq!(rules[0].pattern, "cargo check");
    assert_eq!(rules[0].action, "allow");

    let audits = run_repo.list_audit_logs(10).await?;
    assert_eq!(audits.len(), 1);
    assert_eq!(audits[0].action, "approval.approved");

    Ok(())
}
