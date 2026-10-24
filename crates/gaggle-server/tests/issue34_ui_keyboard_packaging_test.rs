use anyhow::Result;
use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode, header},
};
use gaggle_db::connect_in_memory;
use gaggle_server::{build_router, initialize_server_state};
use serde_json::Value;
use tower::ServiceExt;

async fn response_text(response: axum::response::Response) -> Result<String> {
    let bytes = to_bytes(response.into_body(), 512 * 1024).await?;
    Ok(String::from_utf8(bytes.to_vec())?)
}

async fn response_json(response: axum::response::Response) -> Result<Value> {
    let bytes = to_bytes(response.into_body(), 64 * 1024).await?;
    Ok(serde_json::from_slice(&bytes)?)
}

#[tokio::test]
async fn test_index_serves_keyboard_first_accessible_shell() -> Result<()> {
    let pool = connect_in_memory().await?;
    let state = initialize_server_state(pool).await?;
    let app = build_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/")
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok()),
        Some("text/html; charset=utf-8")
    );

    let html = response_text(response).await?;

    // Screen-reader landmarks and live regions.
    assert!(html.contains("role=\"navigation\" aria-label=\"Sidebar\""));
    assert!(html.contains("role=\"main\" aria-label=\"Main Workspace\""));
    assert!(html.contains("role=\"complementary\" aria-label=\"Contextual Details\""));
    assert!(html.contains("role=\"log\" aria-live=\"polite\""));
    assert!(html.contains("role=\"alert\" aria-live=\"assertive\""));

    // Global keyboard shortcuts and native text-editing protection.
    assert!(html.contains("event.metaKey || event.ctrlKey"));
    assert!(html.contains("shortcutKey === 'k'"));
    assert!(html.contains("shortcutKey === 'n'"));
    assert!(html.contains("shortcutKey === 'b'"));
    assert!(html.contains("shortcutKey === 'i'"));
    assert!(html.contains("shortcutKey === ','"));
    assert!(html.contains("event.key === 'Escape'"));
    assert!(html.contains("NATIVE_EDITING_SHORTCUTS"));
    assert!(html.contains("function trapFocusWithin("));
    assert!(html.contains("focusElement(lastFocusedBeforeSettings)"));
    assert!(html.contains("focusElement(lastFocusedBeforeQuickSwitcher)"));
    assert!(html.contains("focusElement(lastFocusedBeforeAddProject)"));

    // Quick switcher and in-app shortcut cheatsheet.
    assert!(html.contains("id=\"quick-switcher-modal\""));
    assert!(html.contains("role=\"dialog\" aria-modal=\"true\""));
    assert!(html.contains("aria-controls=\"quick-switcher-results\""));
    assert!(html.contains("id=\"tab-keyboard\""));
    assert!(html.contains("id=\"panel-keyboard\""));
    assert!(html.contains("id=\"keyboard-shortcuts-table\""));
    assert!(html.contains("<kbd>Cmd/Ctrl</kbd> + <kbd>K</kbd>"));
    assert!(html.contains("<kbd>Cmd/Ctrl</kbd> + <kbd>N</kbd>"));
    assert!(html.contains("<kbd>Cmd/Ctrl</kbd> + <kbd>B</kbd>"));
    assert!(html.contains("<kbd>Cmd/Ctrl</kbd> + <kbd>I</kbd>"));
    assert!(html.contains("<kbd>Esc</kbd>"));

    // Sidebar and inspector toggles exist for mouse and keyboard users.
    assert!(html.contains("id=\"btn-toggle-sidebar\""));
    assert!(html.contains("id=\"btn-show-sidebar\""));
    assert!(html.contains("id=\"btn-toggle-inspector\""));
    assert!(html.contains("id=\"btn-show-inspector\""));

    Ok(())
}

#[tokio::test]
async fn test_health_endpoint_supports_packaging_smoke_test() -> Result<()> {
    let pool = connect_in_memory().await?;
    let state = initialize_server_state(pool).await?;
    let app = build_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/health")
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::OK);

    let health = response_json(response).await?;
    assert_eq!(health["status"], "ok");
    assert_eq!(health["service"], "gaggle");

    Ok(())
}
