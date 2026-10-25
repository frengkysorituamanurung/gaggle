use gaggle_db::models::Extension;
use gaggle_mcp::McpConnectionManager;
fn mock_extension(script_path: &std::path::Path) -> Extension {
    Extension {
        id: "ext-test".into(),
        name: "mock".into(),
        description: None,
        kind: "stdio".into(),
        version: "1".into(),
        source: "local".into(),
        command: Some("/usr/bin/python3".into()),
        args: Some(
            serde_json::to_string(&vec![script_path.to_string_lossy().to_string()]).unwrap(),
        ),
        url: None,
        env_secret_refs: None,
        timeout_secs: 10,
        install_path: None,
        manifest: None,
        checksum: None,
        bundled: 0,
        enabled: 1,
        installed_at: 0,
    }
}

fn write_mock_script() -> std::path::PathBuf {
    let dir = std::env::temp_dir();
    let script_path = dir.join(format!("mock_mcp_cm_{}.py", uuid::Uuid::new_v4()));
    let script = r#"
import sys, json
for line in sys.stdin:
    line=line.strip()
    if not line:
        continue
    req=json.loads(line)
    method=req.get("method")
    rid=req.get("id")
    if rid is None:
        continue
    if method=="initialize":
        resp={"jsonrpc":"2.0","id":rid,"result":{"protocolVersion":"2024-11-05","capabilities":{"tools":{}},"serverInfo":{"name":"mock"}}}
    elif method=="tools/list":
        resp={"jsonrpc":"2.0","id":rid,"result":{"tools":[{"name":"echo","description":"echo tool","inputSchema":{"type":"object"},"annotations":{"readOnlyHint":True}}]}}
    elif method=="tools/call":
        args=req["params"]["arguments"]
        resp={"jsonrpc":"2.0","id":rid,"result":{"content":[{"type":"text","text":args.get("msg")}],"isError":False}}
    else:
        resp={"jsonrpc":"2.0","id":rid,"result":{}}
    sys.stdout.write(json.dumps(resp)+"\n")
    sys.stdout.flush()
"#;
    std::fs::write(&script_path, script).unwrap();
    script_path
}

#[tokio::test]
async fn connection_reuse_returns_same_client() {
    let script_path = write_mock_script();
    let ext = mock_extension(&script_path);
    let manager = McpConnectionManager::new();

    let c1 = manager.get_or_connect(&ext).await.expect("connect 1");
    let c2 = manager.get_or_connect(&ext).await.expect("connect 2");
    assert!(std::sync::Arc::ptr_eq(&c1, &c2));

    manager.shutdown_all().await.ok();
    let _ = std::fs::remove_file(script_path);
}

#[tokio::test]
async fn health_check_and_invalidate() {
    let script_path = write_mock_script();
    let ext = mock_extension(&script_path);
    let manager = McpConnectionManager::new();

    let _ = manager.get_or_connect(&ext).await.expect("connect");
    assert!(manager.health_check(&ext.id).await);

    manager.invalidate(&ext.id).await.expect("invalidate");
    assert!(!manager.health_check(&ext.id).await);

    let _ = std::fs::remove_file(script_path);
}

#[tokio::test]
async fn shutdown_all_clears_connections() {
    let script_path = write_mock_script();
    let ext = mock_extension(&script_path);
    let manager = McpConnectionManager::new();

    let _ = manager.get_or_connect(&ext).await.expect("connect");
    assert!(manager.health_check(&ext.id).await);

    manager.shutdown_all().await.expect("shutdown");
    assert!(!manager.health_check(&ext.id).await);

    let _ = std::fs::remove_file(script_path);
}

#[tokio::test]
async fn get_or_connect_accepts_extension_value() {
    let script_path = write_mock_script();
    let ext = mock_extension(&script_path);
    let manager = McpConnectionManager::new();

    let c = manager.get_or_connect(ext.clone()).await.expect("connect");
    let tools = c.list_tools().await.expect("list tools");
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0].name, "echo");

    manager.shutdown_all().await.ok();
    let _ = std::fs::remove_file(script_path);
}

#[tokio::test]
async fn health_check_on_client() {
    let script_path = write_mock_script();
    let ext = mock_extension(&script_path);
    let manager = McpConnectionManager::new();

    let client = manager.get_or_connect(&ext).await.expect("connect");
    assert!(manager.health_check(&client).await);

    manager.shutdown_all().await.ok();
    let _ = std::fs::remove_file(script_path);
}
