use anyhow::Result;
use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use gaggle_db::{
    connect_in_memory,
    connection::new_id,
    models::{CommandRule, Extension},
};
use gaggle_server::{build_router, initialize_server_state};
use serde_json::{Value, json};
use tower::ServiceExt;

async fn response_json(response: axum::response::Response) -> Result<Value> {
    let bytes = to_bytes(response.into_body(), 256 * 1024).await?;
    Ok(serde_json::from_slice(&bytes)?)
}

#[tokio::test]
async fn test_provider_model_extension_diagnostics_and_permission_apis() -> Result<()> {
    let temp_root = std::env::temp_dir().join(format!("gaggle_issue19_{}", new_id()));
    std::fs::create_dir_all(&temp_root)?;

    let pool = connect_in_memory().await?;
    let state = initialize_server_state(pool).await?;
    let app = build_router(state);

    // --- Extensions: list, inventory and toggle ---------------------------------
    let extensions_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/extensions")
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(extensions_res.status(), StatusCode::OK);
    let extensions: Vec<Extension> = serde_json::from_value(response_json(extensions_res).await?)?;
    assert!(
        extensions.iter().any(|ext| ext.name == "gaggle.fs"),
        "bundled gaggle.fs extension should be registered"
    );

    let inventory_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/extensions/inventory")
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(inventory_res.status(), StatusCode::OK);
    let inventory = response_json(inventory_res).await?;
    assert_eq!(inventory["recommended_max_active_tools"], 25);
    assert!(inventory["active_tool_count"].as_i64().unwrap_or(0) > 0);
    assert_eq!(inventory["exceeds_recommended_tool_limit"], false);
    assert!(inventory["extensions"].as_array().unwrap().len() >= 5);

    let first_extension_id = extensions
        .first()
        .expect("at least one bundled extension")
        .id
        .clone();
    let disable_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(format!("/api/extensions/{first_extension_id}"))
                .header("content-type", "application/json")
                .body(Body::from(json!({ "enabled": false }).to_string()))?,
        )
        .await?;
    assert_eq!(disable_res.status(), StatusCode::OK);
    let disabled: Extension = serde_json::from_value(response_json(disable_res).await?)?;
    assert_eq!(disabled.enabled, 0);

    let enable_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(format!("/api/extensions/{first_extension_id}"))
                .header("content-type", "application/json")
                .body(Body::from(json!({ "enabled": true }).to_string()))?,
        )
        .await?;
    assert_eq!(enable_res.status(), StatusCode::OK);

    // --- Providers, credential presence and models ------------------------------
    let raw_secret = "sk-live-DO-NOT-LEAK-1234567890";
    let provider_payload = json!({
        "kind": "openai",
        "name": "OpenAI test",
        "base_url": "https://api.openai.com/v1",
        "auth_kind": "api_key",
        "api_key": raw_secret
    })
    .to_string();
    let create_provider_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/providers")
                .header("content-type", "application/json")
                .body(Body::from(provider_payload))?,
        )
        .await?;
    assert_eq!(create_provider_res.status(), StatusCode::CREATED);
    let created_provider = response_json(create_provider_res).await?;
    assert_eq!(created_provider["has_configured_secret"], true);
    let provider_id = created_provider["provider"]["id"]
        .as_str()
        .expect("provider id")
        .to_string();
    let secret_ref = created_provider["provider"]["secret_ref"]
        .as_str()
        .expect("generated secret_ref")
        .to_string();
    assert_ne!(secret_ref, raw_secret);
    assert!(!secret_ref.is_empty());

    let providers_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/providers")
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(providers_res.status(), StatusCode::OK);
    let providers_body = response_json(providers_res).await?.to_string();
    assert!(!providers_body.contains(raw_secret));
    assert!(providers_body.contains("has_configured_secret"));

    let model_payload = json!({
        "model_key": "gpt-4o-mini",
        "display_name": "GPT-4o mini",
        "context_window": 128000,
        "supports_tools": true
    })
    .to_string();
    let create_model_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/providers/{provider_id}/models"))
                .header("content-type", "application/json")
                .body(Body::from(model_payload))?,
        )
        .await?;
    assert_eq!(create_model_res.status(), StatusCode::CREATED);

    let models_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/providers/{provider_id}/models"))
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(models_res.status(), StatusCode::OK);
    let models = response_json(models_res).await?;
    assert_eq!(models[0]["model_key"], "gpt-4o-mini");

    // --- Diagnostics: live report, redacted -------------------------------------
    let diagnostics_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/diagnostics")
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(diagnostics_res.status(), StatusCode::OK);
    let diagnostics = response_json(diagnostics_res).await?;
    assert_eq!(diagnostics["service"], "gaggle");
    assert_eq!(diagnostics["db_healthy"], true);
    assert!(matches!(
        diagnostics["journal_mode"].as_str(),
        Some("wal") | Some("memory")
    ));
    assert_eq!(diagnostics["foreign_keys_enabled"], true);
    assert!(diagnostics["applied_migrations"].as_i64().unwrap_or(0) > 0);
    assert_eq!(diagnostics["active_projects_count"], 0);
    assert!(
        diagnostics["enabled_extensions_count"]
            .as_i64()
            .unwrap_or(0)
            >= 5
    );
    let diagnostics_body = diagnostics.to_string();
    assert!(!diagnostics_body.contains(raw_secret));
    assert!(!diagnostics_body.contains("secret_ref"));

    // --- Project defaults, command rules and tool permissions --------------------
    let project_payload = json!({
        "name": "Issue 19 workspace",
        "root_path": temp_root.to_string_lossy(),
        "vcs_kind": "none"
    })
    .to_string();
    let create_project_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/projects")
                .header("content-type", "application/json")
                .body(Body::from(project_payload))?,
        )
        .await?;
    assert_eq!(create_project_res.status(), StatusCode::CREATED);
    let project = response_json(create_project_res).await?;
    let project_id = project["id"].as_str().expect("project id").to_string();

    let update_project_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(format!("/api/projects/{project_id}"))
                .header("content-type", "application/json")
                .body(Body::from(json!({ "default_access": "full" }).to_string()))?,
        )
        .await?;
    assert_eq!(update_project_res.status(), StatusCode::OK);
    let updated_project = response_json(update_project_res).await?;
    assert_eq!(updated_project["default_access"], "full");

    let invalid_access_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(format!("/api/projects/{project_id}"))
                .header("content-type", "application/json")
                .body(Body::from(json!({ "default_access": "root" }).to_string()))?,
        )
        .await?;
    assert_eq!(invalid_access_res.status(), StatusCode::BAD_REQUEST);

    let create_rule_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/projects/{project_id}/command-rules"))
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "pattern": "rm -rf", "match_kind": "prefix", "action": "deny" })
                        .to_string(),
                ))?,
        )
        .await?;
    assert_eq!(create_rule_res.status(), StatusCode::CREATED);

    let rules_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/projects/{project_id}/command-rules"))
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(rules_res.status(), StatusCode::OK);
    let rules: Vec<CommandRule> = serde_json::from_value(response_json(rules_res).await?)?;
    assert_eq!(rules.len(), 1);
    assert_eq!(rules[0].pattern, "rm -rf");
    assert_eq!(rules[0].action, "deny");

    let tool_permissions_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/projects/{project_id}/tool-permissions"))
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(tool_permissions_res.status(), StatusCode::OK);
    let tool_permissions = response_json(tool_permissions_res).await?;
    let first_tool_id = tool_permissions[0]["extension_tool_id"]
        .as_str()
        .expect("extension tool id")
        .to_string();

    let set_permission_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/projects/{project_id}/tool-permissions"))
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "extension_tool_id": first_tool_id, "level": "never_allow" })
                        .to_string(),
                ))?,
        )
        .await?;
    assert_eq!(set_permission_res.status(), StatusCode::OK);
    let updated_permission = response_json(set_permission_res).await?;
    assert_eq!(updated_permission["level"], "never_allow");

    let invalid_permission_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/projects/{project_id}/tool-permissions"))
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "extension_tool_id": first_tool_id, "level": "sometimes" }).to_string(),
                ))?,
        )
        .await?;
    assert_eq!(invalid_permission_res.status(), StatusCode::BAD_REQUEST);

    let refreshed_permissions_res = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/projects/{project_id}/tool-permissions"))
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(refreshed_permissions_res.status(), StatusCode::OK);
    let refreshed_permissions = response_json(refreshed_permissions_res).await?;
    let matched = refreshed_permissions
        .as_array()
        .unwrap()
        .iter()
        .find(|status| status["extension_tool_id"] == first_tool_id.as_str())
        .expect("tool permission entry");
    assert_eq!(matched["level"], "never_allow");

    std::fs::remove_dir_all(&temp_root)?;
    Ok(())
}
