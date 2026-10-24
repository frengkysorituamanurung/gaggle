use gaggle_mcp::McpClient;
use serde_json::json;
use std::time::Duration;
use tokio::net::TcpListener;

#[tokio::test]
async fn sse_roundtrip() {
    use axum::{
        Router,
        extract::State,
        response::sse::{Event, Sse},
        routing::{get, post},
    };
    use futures::StreamExt;
    use futures::stream;
    use std::convert::Infallible;
    use tokio::sync::broadcast;

    #[derive(Clone)]
    struct S {
        tx: broadcast::Sender<String>,
    }

    async fn sse_handler(
        State(s): State<S>,
    ) -> Sse<impl futures::Stream<Item = Result<Event, Infallible>>> {
        let rx = s.tx.subscribe();
        let stream =
            stream::once(async { Ok(Event::default().event("endpoint").data("/message")) }).chain(
                futures::stream::unfold(rx, |mut rx| async move {
                    match rx.recv().await {
                        Ok(data) => Some((Ok(Event::default().event("message").data(data)), rx)),
                        Err(_) => None,
                    }
                }),
            );
        Sse::new(stream)
    }

    async fn post_handler(
        State(s): State<S>,
        axum::Json(req): axum::Json<serde_json::Value>,
    ) -> axum::http::StatusCode {
        let method = req["method"].as_str().unwrap_or("");
        let id = req["id"].clone();
        let resp = match method {
            "initialize" => {
                json!({"jsonrpc":"2.0","id":id,"result":{"protocolVersion":"2024-11-05","capabilities":{"tools":{}},"serverInfo":{"name":"mock"}}})
            }
            "tools/list" => {
                json!({"jsonrpc":"2.0","id":id,"result":{"tools":[{"name":"sse_tool","description":"sse","inputSchema":{"type":"object"}}]}})
            }
            "tools/call" => {
                json!({"jsonrpc":"2.0","id":id,"result":{"content":[{"type":"text","text":"sse-ok"}],"isError":false}})
            }
            _ => json!({"jsonrpc":"2.0","id":id,"result":{}}),
        };
        let _ = s.tx.send(resp.to_string());
        axum::http::StatusCode::ACCEPTED
    }

    let (tx, _rx) = broadcast::channel(16);
    let state = S { tx };
    let app = Router::new()
        .route("/sse", get(sse_handler))
        .route("/message", post(post_handler))
        .with_state(state);
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    // give server time
    tokio::time::sleep(Duration::from_millis(50)).await;

    let url = format!("http://{}/sse", addr);
    let client = McpClient::connect_sse(url, Duration::from_secs(10))
        .await
        .expect("connect");
    // Wait for endpoint event processed
    tokio::time::sleep(Duration::from_millis(100)).await;

    let init = client.initialize().await.expect("init");
    assert_eq!(init.protocol_version, "2024-11-05");

    let tools = client.list_tools().await.expect("list");
    assert_eq!(tools[0].name, "sse_tool");

    let res = client.call_tool("sse_tool", json!({})).await.expect("call");
    assert_eq!(res.text(), "sse-ok");

    client.close().await.ok();
}
