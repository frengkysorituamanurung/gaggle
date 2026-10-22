use anyhow::Result;
use gaggle_core::WorkspacePathPolicy;
use gaggle_db::connection::new_id;
use gaggle_tools::{
    DEFAULT_MAX_DIFF_BYTES, DEFAULT_MAX_LIST_ENTRIES, DEFAULT_MAX_READ_BYTES,
    DEFAULT_MAX_SEARCH_MATCHES, FsToolError, fs_list, fs_read, fs_search, git_diff, git_status,
};
use std::process::Command;

#[test]
fn test_fs_list_read_search_and_security_limits() -> Result<()> {
    let temp_root = std::env::temp_dir().join(format!("gaggle_m2_fs_{}", new_id()));
    std::fs::create_dir_all(temp_root.join("src"))?;
    std::fs::create_dir_all(temp_root.join("target"))?;

    std::fs::write(
        temp_root.join("src/lib.rs"),
        "pub fn greet() -> &'static str {\n    \"hello gaggle\"\n}\n",
    )?;
    // Sensitive file that must be automatically excluded from fs_list and fs_search
    std::fs::write(temp_root.join(".env"), "API_KEY=secret_value\n")?;
    // Ignored build directory file
    std::fs::write(temp_root.join("target/ignored.txt"), "hello gaggle\n")?;
    // Binary file containing NUL bytes
    std::fs::write(temp_root.join("src/blob.bin"), [0x89, 0x50, 0x00, 0x01])?;

    let policy = WorkspacePathPolicy::new(&temp_root)?;

    // 1. fs_list includes src, excludes .env and target
    let entries = fs_list(&policy, None, DEFAULT_MAX_LIST_ENTRIES)?;
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].relative_path, "src");
    assert!(entries[0].is_dir);

    // 2. fs_read succeeds on valid UTF-8 source file
    let read_out = fs_read(&policy, "src/lib.rs", DEFAULT_MAX_READ_BYTES)?;
    assert_eq!(read_out.relative_path, "src/lib.rs");
    assert!(read_out.content.contains("hello gaggle"));

    // 3. fs_read enforces maximum byte size limit
    let size_err = fs_read(&policy, "src/lib.rs", 8).unwrap_err();
    assert!(matches!(size_err, FsToolError::FileTooLarge { .. }));

    // 4. fs_read rejects binary files containing NUL bytes
    let bin_err = fs_read(&policy, "src/blob.bin", DEFAULT_MAX_READ_BYTES).unwrap_err();
    assert!(matches!(bin_err, FsToolError::BinaryFileRejected { .. }));

    // 5. fs_read rejects sensitive .env file
    let env_err = fs_read(&policy, ".env", DEFAULT_MAX_READ_BYTES).unwrap_err();
    assert!(matches!(env_err, FsToolError::Policy(_)));

    // 6. fs_search finds matches in src/lib.rs while skipping target/ and .env
    let matches = fs_search(&policy, "hello gaggle", DEFAULT_MAX_SEARCH_MATCHES)?;
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].relative_path, "src/lib.rs");
    assert_eq!(matches[0].line_number, 2);

    let secret_matches = fs_search(&policy, "secret_value", DEFAULT_MAX_SEARCH_MATCHES)?;
    assert!(secret_matches.is_empty());

    let _ = std::fs::remove_dir_all(&temp_root);
    Ok(())
}

#[test]
fn test_git_status_and_diff_inspection() -> Result<()> {
    let temp_root = std::env::temp_dir().join(format!("gaggle_m2_git_{}", new_id()));
    std::fs::create_dir_all(&temp_root)?;

    let policy = WorkspacePathPolicy::new(&temp_root)?;

    // 1. Non-git directory returns is_git_repo = false gracefully
    let non_git = git_status(&policy)?;
    assert!(!non_git.is_git_repo);

    // 2. Initialize a Git repo, commit one file, modify it, and add an untracked file
    assert!(
        Command::new("git")
            .arg("-C")
            .arg(&temp_root)
            .args(["init", "-b", "main"])
            .status()?
            .success()
    );
    assert!(
        Command::new("git")
            .arg("-C")
            .arg(&temp_root)
            .args(["config", "user.email", "test@gaggle.local"])
            .status()?
            .success()
    );
    assert!(
        Command::new("git")
            .arg("-C")
            .arg(&temp_root)
            .args(["config", "user.name", "Gaggle Test"])
            .status()?
            .success()
    );

    let tracked_file = temp_root.join("README.md");
    std::fs::write(&tracked_file, "# Initial\n")?;
    assert!(
        Command::new("git")
            .arg("-C")
            .arg(&temp_root)
            .args(["add", "README.md"])
            .status()?
            .success()
    );
    assert!(
        Command::new("git")
            .arg("-C")
            .arg(&temp_root)
            .args(["commit", "-m", "init"])
            .status()?
            .success()
    );

    let clean_status = git_status(&policy)?;
    assert!(clean_status.is_git_repo);
    assert_eq!(clean_status.branch.as_deref(), Some("main"));
    assert!(clean_status.is_clean);

    // Modify README.md and create untracked notes.txt
    std::fs::write(&tracked_file, "# Initial\nAdded line\n")?;
    std::fs::write(temp_root.join("notes.txt"), "untracked note\n")?;

    let dirty_status = git_status(&policy)?;
    assert!(!dirty_status.is_clean);
    assert_eq!(dirty_status.changed_files.len(), 2);
    assert!(
        dirty_status
            .changed_files
            .iter()
            .any(|f| f.path == "README.md" && !f.untracked)
    );
    assert!(
        dirty_status
            .changed_files
            .iter()
            .any(|f| f.path == "notes.txt" && f.untracked)
    );

    // Inspect unified diff
    let diff = git_diff(&policy, Some("README.md"), DEFAULT_MAX_DIFF_BYTES)?;
    assert!(diff.is_git_repo);
    assert!(!diff.truncated);
    assert!(diff.patch.contains("+Added line"));

    // Path traversal in git_diff is rejected by WorkspacePathPolicy
    let traversal_err = git_diff(&policy, Some("../outside.txt"), DEFAULT_MAX_DIFF_BYTES);
    assert!(traversal_err.is_err());

    let _ = std::fs::remove_dir_all(&temp_root);
    Ok(())
}
