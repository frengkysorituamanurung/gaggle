use anyhow::Result;
use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use gaggle_db::{connect_in_memory, connection::new_id};
use gaggle_server::{ProjectStatusResponse, build_router, initialize_server_state};
use gaggle_tools::FsEntry;
use tower::ServiceExt;

#[tokio::test]
async fn test_project_registration_validation_status_and_file_listing() -> Result<()> {
    let temp_root = std::env::temp_dir().join(format!("gaggle_srv_p32_{}", new_id()));
    std::fs::create_dir_all(temp_root.join("src"))?;
    std::fs::write(temp_root.join("src/main.rs"), "fn main() {}\n")?;

    let pool = connect_in_memory().await?;
    let state = initialize_server_state(pool).await?;
    let app = build_router(state);

    // 1. Registering a restricted system path (`/`) returns 400 Bad Request
    let bad_req = Request::builder()
        .method("POST")
        .uri("/api/projects")
        .header("content-type", "application/json")
        .body(Body::from(
            r#"{"name":"root","root_path":"/","vcs_kind":"none"}"#,
        ))?;
    let bad_resp = app.clone().oneshot(bad_req).await?;
    assert_eq!(bad_resp.status(), StatusCode::BAD_REQUEST);

    // 2. Registering a non-existent path returns 400 Bad Request
    let missing_req = Request::builder()
        .method("POST")
        .uri("/api/projects")
        .header("content-type", "application/json")
        .body(Body::from(
            r#"{"name":"ghost","root_path":"/nonexistent/gaggle/path/12345","vcs_kind":"none"}"#,
        ))?;
    let missing_resp = app.clone().oneshot(missing_req).await?;
    assert_eq!(missing_resp.status(), StatusCode::BAD_REQUEST);

    // 3. Registering a valid project directory returns 201 Created with canonical root_path and path_exists = true
    let create_payload = serde_json::json!({
        "name": "Workspace 32",
        "root_path": temp_root.to_string_lossy(),
        "vcs_kind": "none"
    })
    .to_string();
    let create_req = Request::builder()
        .method("POST")
        .uri("/api/projects")
        .header("content-type", "application/json")
        .body(Body::from(create_payload))?;
    let create_resp = app.clone().oneshot(create_req).await?;
    assert_eq!(create_resp.status(), StatusCode::CREATED);

    let body_bytes = to_bytes(create_resp.into_body(), 64 * 1024).await?;
    let created: ProjectStatusResponse = serde_json::from_slice(&body_bytes)?;
    assert_eq!(created.project.name, "Workspace 32");
    assert!(created.path_exists);

    // 4. Listing files via GET /api/projects/{id}/files returns validated workspace entries
    let files_req = Request::builder()
        .method("GET")
        .uri(format!("/api/projects/{}/files", created.project.id))
        .body(Body::empty())?;
    let files_resp = app.clone().oneshot(files_req).await?;
    assert_eq!(files_resp.status(), StatusCode::OK);
    let files_bytes = to_bytes(files_resp.into_body(), 64 * 1024).await?;
    let entries: Vec<FsEntry> = serde_json::from_slice(&files_bytes)?;
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].relative_path, "src");
    assert!(entries[0].is_dir);

    // 5. Removing the directory on disk causes GET /api/projects to report `path_exists: false`
    std::fs::remove_dir_all(&temp_root)?;
    let list_req = Request::builder()
        .method("GET")
        .uri("/api/projects")
        .body(Body::empty())?;
    let list_resp = app.oneshot(list_req).await?;
    assert_eq!(list_resp.status(), StatusCode::OK);
    let list_bytes = to_bytes(list_resp.into_body(), 64 * 1024).await?;
    let projects: Vec<ProjectStatusResponse> = serde_json::from_slice(&list_bytes)?;
    assert_eq!(projects.len(), 1);
    assert!(!projects[0].path_exists);

    Ok(())
}
