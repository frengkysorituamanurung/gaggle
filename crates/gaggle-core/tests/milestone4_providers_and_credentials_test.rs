use anyhow::Result;
use gaggle_core::{
    CredentialError, CredentialStore, InMemoryCredentialStore, MockProviderAdapter,
    ModelCompletionRequest, ModelCompletionResponse, ModelProviderAdapter, OpenAiCompatibleAdapter,
    ProviderError, ProviderMessage, SecretString, TokenUsage, redact_sensitive_text,
    validate_provider_base_url,
};
use gaggle_db::{connect_in_memory, repositories::providers::ProviderRepository};
use std::sync::Arc;

#[test]
fn test_secret_redaction_and_credential_store_isolation() -> Result<()> {
    let store = InMemoryCredentialStore::new();
    let secret = SecretString::new("sk-live-super-secret-token-12345")?;

    // Debug and Display never expose the raw secret value
    assert_eq!(format!("{secret}"), "[REDACTED]");
    assert_eq!(format!("{secret:?}"), "SecretString([REDACTED])");

    // Redaction helper strips raw secret from error/diagnostic strings
    let raw_err = "Upstream connection failed with header Authorization: Bearer sk-live-super-secret-token-12345";
    let safe_err = redact_sensitive_text(raw_err, Some(&secret));
    assert!(!safe_err.contains("sk-live-super-secret-token-12345"));
    assert!(safe_err.contains("[REDACTED]"));

    // Store, lookup, and delete by secret_ref
    store.set_secret("openai_main", secret)?;
    assert!(store.has_secret("openai_main"));
    let fetched = store.get_secret("openai_main")?;
    assert_eq!(fetched.expose_secret(), "sk-live-super-secret-token-12345");

    store.delete_secret("openai_main")?;
    assert!(!store.has_secret("openai_main"));
    assert_eq!(
        store.get_secret("openai_main").unwrap_err(),
        CredentialError::NotFound("openai_main".to_string())
    );

    Ok(())
}

#[test]
fn test_provider_url_validation_and_openai_response_normalization() -> Result<()> {
    // 1. Valid HTTP/HTTPS URLs pass and strip trailing slash
    assert_eq!(
        validate_provider_base_url("https://api.openai.com/v1/")?,
        "https://api.openai.com/v1"
    );
    assert_eq!(
        validate_provider_base_url("http://127.0.0.1:11434/v1")?,
        "http://127.0.0.1:11434/v1"
    );

    // 2. Non-HTTP schemes are rejected
    assert!(matches!(
        validate_provider_base_url("file:///etc/passwd").unwrap_err(),
        ProviderError::InvalidBaseUrl { .. }
    ));
    assert!(matches!(
        validate_provider_base_url("ftp://example.com").unwrap_err(),
        ProviderError::InvalidBaseUrl { .. }
    ));

    // 3. OpenAI-compatible JSON payload with tool calls and usage is normalized
    let raw_json = r#"{
        "id": "chatcmpl-123",
        "choices": [{
            "index": 0,
            "message": {
                "role": "assistant",
                "content": "Let me read Cargo.toml first.",
                "tool_calls": [{
                    "id": "call_abc123",
                    "type": "function",
                    "function": {
                        "name": "fs.read",
                        "arguments": "{\"path\":\"Cargo.toml\"}"
                    }
                }]
            }
        }],
        "usage": {
            "prompt_tokens": 120,
            "completion_tokens": 35
        }
    }"#;

    let parsed = OpenAiCompatibleAdapter::parse_chat_completion_json("openai", raw_json)?;
    assert_eq!(parsed.text, "Let me read Cargo.toml first.");
    assert_eq!(parsed.tool_calls.len(), 1);
    assert_eq!(parsed.tool_calls[0].id, "call_abc123");
    assert_eq!(parsed.tool_calls[0].tool_name, "fs.read");
    assert_eq!(
        parsed.tool_calls[0].arguments_json,
        r#"{"path":"Cargo.toml"}"#
    );
    assert_eq!(parsed.usage.input_tokens, 120);
    assert_eq!(parsed.usage.output_tokens, 35);

    // 4. Malformed JSON is rejected with structured error
    let malformed = OpenAiCompatibleAdapter::parse_chat_completion_json("openai", "{not-json}");
    assert!(matches!(
        malformed.unwrap_err(),
        ProviderError::MalformedResponse { .. }
    ));

    Ok(())
}

#[tokio::test]
async fn test_provider_repository_and_mock_adapter() -> Result<()> {
    let pool = connect_in_memory().await?;
    let repo = ProviderRepository::new(pool);

    // SQLite only stores secret_ref, never the API key itself
    let provider = repo
        .create_provider(
            "openai",
            "OpenAI Production",
            Some("https://api.openai.com/v1"),
            "api_key",
            Some("keychain:gaggle/openai_prod"),
        )
        .await?;
    assert_eq!(
        provider.secret_ref.as_deref(),
        Some("keychain:gaggle/openai_prod")
    );

    let model = repo
        .create_model(
            &provider.id,
            "gpt-4.1",
            "GPT-4.1",
            128_000,
            true,
            true,
            false,
            Some(2.0),
            Some(8.0),
        )
        .await?;
    assert_eq!(model.model_key, "gpt-4.1");

    let models = repo.list_models_by_provider(&provider.id).await?;
    assert_eq!(models.len(), 1);

    // MockProviderAdapter deterministic completion
    let mock = MockProviderAdapter::with_response(
        "mock-openai",
        ModelCompletionResponse {
            text: "Mock assistant response".to_string(),
            tool_calls: vec![],
            usage: TokenUsage {
                input_tokens: 10,
                output_tokens: 5,
                cached_tokens: 0,
            },
        },
    );

    let req = ModelCompletionRequest {
        model_key: model.model_key,
        messages: vec![ProviderMessage {
            role: "user".to_string(),
            content: "Hello".to_string(),
        }],
        tools: vec![],
        max_output_tokens: Some(256),
    };
    let resp = mock.complete(&req).await?;
    assert_eq!(resp.text, "Mock assistant response");
    assert_eq!(resp.usage.input_tokens, 10);

    // Missing credential in OpenAiCompatibleAdapter returns CredentialError::NotFound
    let store = Arc::new(InMemoryCredentialStore::new());
    let adapter = OpenAiCompatibleAdapter::new(
        "openai",
        "https://api.openai.com/v1",
        Some("missing_ref".to_string()),
        store,
        5000,
    )?;
    let err = adapter.complete(&req).await.unwrap_err();
    assert_eq!(
        err,
        ProviderError::Credential(CredentialError::NotFound("missing_ref".to_string()))
    );

    Ok(())
}
