use gaggle_mcp::{McpClient, validate_mcp_url};
use serde_json::json;
use std::time::Duration;
use tokio::net::TcpListener;

#[tokio::test]
async fn stdio_transport_roundtrip() {
    let dir = std::env::temp_dir();
    let script_path = dir.join(format!("mock_mcp_{}.py", uuid::Uuid::new_v4()));
    let script = r#"
import sys, json
for line in sys.stdin:
    line=line.strip()
    if not line:
        continue
    req=json.loads(line)
    method=req.get("method")
    rid=req.get("id")
    if method=="initialize":
        if rid is None:
            sys.exit(0)
        resp={"jsonrpc":"2.0","id":rid,"result":{"protocolVersion":"2024-11-05","capabilities":{"tools":{}},"serverInfo":{"name":"mock"}}}
    elif method=="tools/list":
        resp={"jsonrpc":"2.0","id":req["id"],"result":{"tools":[{"name":"echo","description":"echo tool","inputSchema":{"type":"object","properties":{"msg":{"type":"string"}}},"annotations":{"readOnlyHint":True}}]}}
    elif method=="tools/call":
        args=req["params"]["arguments"]
        resp={"jsonrpc":"2.0","id":req["id"],"result":{"content":[{"type":"text","text":args.get("msg")}],"isError":False}}
    else:
        resp={"jsonrpc":"2.0","id":req["id"],"result":{}}
    sys.stdout.write(json.dumps(resp)+"\n")
    sys.stdout.flush()
"#;
    std::fs::write(&script_path, script).unwrap();

    let client = McpClient::connect_stdio(
        "/usr/bin/python3",
        vec![script_path.to_string_lossy().to_string()],
        std::collections::HashMap::new(),
        Duration::from_secs(10),
    )
    .await
    .expect("connect");

    let init = client.initialize().await.expect("init");
    assert_eq!(init.protocol_version, "2024-11-05");

    let tools = client.list_tools().await.expect("list");
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0].name, "echo");
    assert!(tools[0].is_read_only);

    let res = client
        .call_tool("echo", json!({"msg":"hello"}))
        .await
        .expect("call");
    assert!(!res.is_error);
    assert_eq!(res.text(), "hello");

    client.close().await.ok();
    let _ = std::fs::remove_file(script_path);
}

#[tokio::test]
async fn streamable_http_roundtrip() {
    use axum::{Json, Router, extract::State, routing::post};
    use std::sync::{Arc, Mutex};

    #[derive(Clone)]
    struct S {
        seen: Arc<Mutex<u32>>,
    }

    async fn handler(
        State(s): State<S>,
        Json(req): Json<serde_json::Value>,
    ) -> Json<serde_json::Value> {
        *s.seen.lock().unwrap() += 1;
        let method = req["method"].as_str().unwrap_or("");
        let id = req["id"].clone();
        let resp = match method {
            "initialize" => {
                json!({"jsonrpc":"2.0","id":id,"result":{"protocolVersion":"2024-11-05","capabilities":{"tools":{}},"serverInfo":{"name":"mock"}}})
            }
            "tools/list" => {
                json!({"jsonrpc":"2.0","id":id,"result":{"tools":[{"name":"add","description":"add","inputSchema":{"type":"object"}}]}})
            }
            "tools/call" => {
                json!({"jsonrpc":"2.0","id":id,"result":{"content":[{"type":"text","text":"42"}],"isError":false}})
            }
            _ => json!({"jsonrpc":"2.0","id":id,"result":{}}),
        };
        Json(resp)
    }

    let state = S {
        seen: Arc::new(Mutex::new(0)),
    };
    let app = Router::new()
        .route("/mcp", post(handler))
        .with_state(state.clone());
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let url = format!("http://{}/mcp", addr);
    let client =
        gaggle_mcp::McpClient::connect_streamable_http(url.clone(), Duration::from_secs(10))
            .await
            .expect("connect");
    client.initialize().await.expect("init");
    let tools = client.list_tools().await.expect("list");
    assert_eq!(tools[0].name, "add");
    let res = client.call_tool("add", json!({})).await.expect("call");
    assert_eq!(res.text(), "42");
    client.close().await.ok();
}

#[tokio::test]
async fn validate_mcp_url_blocks_private() {
    assert!(validate_mcp_url("http://192.168.1.1/x").is_err());
    assert!(validate_mcp_url("http://10.0.0.1/x").is_err());
    assert!(validate_mcp_url("http://169.254.169.254/latest").is_err());
    assert!(validate_mcp_url("ftp://example.com").is_err());
    assert!(validate_mcp_url("http://example.com/x").is_ok());
}
