use anyhow::Result;
use gaggle_core::{PermissionDecision, PermissionEngine, ToolCallContext};
use gaggle_db::{
    connect_in_memory,
    repositories::{extensions::ExtensionRepository, permissions::PermissionRepository},
};

#[tokio::test]
async fn test_permission_decision_order() -> Result<()> {
    let pool = connect_in_memory().await?;
    let ext_repo = ExtensionRepository::new(pool.clone());
    let perm_repo = PermissionRepository::new(pool.clone());
    let engine = PermissionEngine::new(pool.clone());

    let ext = ext_repo
        .upsert_builtin("gaggle.shell", "Shell", "0.1.0")
        .await?;
    let shell_tool = ext_repo
        .upsert_tool(&ext.id, "shell.exec", "Exec", r#"{"type":"object"}"#, false)
        .await?;
    let read_tool = ext_repo
        .upsert_tool(&ext.id, "fs.read", "Read", r#"{"type":"object"}"#, true)
        .await?;

    // 1. chat_only rejects even read-only tools
    let ctx_chat_only = ToolCallContext {
        project_id: None,
        permission_mode: "chat_only".to_string(),
        extension_tool: read_tool.clone(),
        args_json: r#"{"path":"README.md"}"#.to_string(),
        shell_command: None,
    };
    assert!(matches!(
        engine.evaluate(&ctx_chat_only).await?,
        PermissionDecision::Deny { .. }
    ));

    // 2. command_rules deny always blocks, even in `full` mode
    perm_repo
        .add_command_rule(None, "rm -rf /", "prefix", "deny")
        .await?;
    let ctx_full_denied = ToolCallContext {
        project_id: None,
        permission_mode: "full".to_string(),
        extension_tool: shell_tool.clone(),
        args_json: r#"{"command":"rm -rf /tmp"}"#.to_string(),
        shell_command: Some("rm -rf /tmp".to_string()),
    };
    assert!(matches!(
        engine.evaluate(&ctx_full_denied).await?,
        PermissionDecision::Deny { .. }
    ));

    // 3. read-only tool is allowed in `ask` mode without approval
    let ctx_read_ask = ToolCallContext {
        project_id: None,
        permission_mode: "ask".to_string(),
        extension_tool: read_tool,
        args_json: r#"{"path":"Cargo.toml"}"#.to_string(),
        shell_command: None,
    };
    assert_eq!(
        engine.evaluate(&ctx_read_ask).await?,
        PermissionDecision::Allow
    );

    // 4. non-read-only tool in `ask` mode requires approval
    let ctx_write_ask = ToolCallContext {
        project_id: None,
        permission_mode: "ask".to_string(),
        extension_tool: shell_tool,
        args_json: r#"{"command":"cargo build"}"#.to_string(),
        shell_command: Some("cargo build".to_string()),
    };
    assert!(matches!(
        engine.evaluate(&ctx_write_ask).await?,
        PermissionDecision::NeedsApproval { .. }
    ));

    Ok(())
}
