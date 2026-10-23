//! ChatGPT OAuth (PKCE) support and an OAuth-backed OpenAI-compatible adapter.
//!
//! Tokens are kept exclusively in the [`CredentialStore`] under a caller-provided secret
//! reference. The database only ever stores that reference, never the token material.

use crate::credentials::{CredentialStore, SecretString, redact_sensitive_text};
use crate::provider::adapter_support::{
    LineAccumulator, OpenAiChunkOutcome, OpenAiSseState, SseFrameAccumulator,
    build_openai_chat_payload, ensure_success_response, map_reqwest_send_error,
};
use crate::provider::{
    ModelCompletionRequest, ModelCompletionResponse, ModelCompletionStream, ModelProviderAdapter,
    ModelStreamEvent, OpenAiCompatibleAdapter, ProviderError, map_provider_http_error, now_unix_ms,
    retry_after_ms, validate_provider_base_url,
};
use async_trait::async_trait;
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use std::{sync::Arc, time::Duration};

const OAUTH_PROVIDER: &str = "chatgpt_oauth";
const TOKEN_REFRESH_SKEW_MS: i64 = 60_000;

/// Persisted OAuth token material. Serialized and stored only inside the OS credential store.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OAuthTokenSet {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at_ms: Option<i64>,
    pub token_type: String,
    pub scope: Option<String>,
}

/// Endpoints and client identity required for the ChatGPT PKCE authorization-code flow.
#[derive(Debug, Clone)]
pub struct ChatGptOAuthConfig {
    pub client_id: String,
    pub auth_url: String,
    pub token_url: String,
    pub redirect_uri: String,
    pub scopes: Vec<String>,
}

/// Authorization URL plus the correlating `state` and PKCE verifier values.
pub struct PkceAuthorization {
    pub url: String,
    pub state: String,
    pub code_verifier: String,
}

/// Response payload returned by an OAuth token endpoint.
#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    #[serde(default)]
    refresh_token: Option<String>,
    #[serde(default)]
    expires_in: Option<i64>,
    #[serde(default)]
    token_type: Option<String>,
    #[serde(default)]
    scope: Option<String>,
}

impl TokenResponse {
    fn into_token_set(self) -> OAuthTokenSet {
        OAuthTokenSet {
            access_token: self.access_token,
            refresh_token: self.refresh_token,
            expires_at_ms: self
                .expires_in
                .map(|seconds| now_unix_ms().saturating_add(seconds.saturating_mul(1000))),
            token_type: self.token_type.unwrap_or_else(|| "Bearer".to_string()),
            scope: self.scope,
        }
    }
}

/// Minimal OAuth 2.0 authorization-code + PKCE client for ChatGPT-style providers.
#[derive(Clone)]
pub struct ChatGptOAuthClient {
    config: ChatGptOAuthConfig,
    store: Arc<dyn CredentialStore>,
    client: reqwest::Client,
    timeout_ms: u64,
}

impl ChatGptOAuthClient {
    pub fn new(
        config: ChatGptOAuthConfig,
        store: Arc<dyn CredentialStore>,
        timeout_ms: u64,
    ) -> Result<Self, ProviderError> {
        validate_provider_base_url(&config.auth_url)?;
        validate_provider_base_url(&config.token_url)?;
        validate_provider_base_url(&config.redirect_uri)?;

        if config.client_id.trim().is_empty() {
            return Err(ProviderError::OAuth {
                provider: OAUTH_PROVIDER.to_string(),
                message: "OAuth client_id must not be empty".to_string(),
            });
        }

        let client = reqwest::Client::builder()
            .timeout(Duration::from_millis(timeout_ms))
            .build()
            .map_err(|e| ProviderError::OAuth {
                provider: OAUTH_PROVIDER.to_string(),
                message: format!("Failed to build OAuth HTTP client: {e}"),
            })?;

        Ok(Self {
            config,
            store,
            client,
            timeout_ms,
        })
    }

    /// Builds an authorization URL using S256 PKCE, a random `state`, and the configured scopes.
    pub fn authorization_url(&self) -> Result<PkceAuthorization, ProviderError> {
        let state = random_urlsafe(32);
        let code_verifier = random_urlsafe(32);
        let code_challenge = pkce_s256_challenge(&code_verifier);

        let mut url =
            reqwest::Url::parse(&self.config.auth_url).map_err(|e| ProviderError::OAuth {
                provider: OAUTH_PROVIDER.to_string(),
                message: format!("Invalid authorization URL: {e}"),
            })?;

        {
            let mut query = url.query_pairs_mut();
            query.append_pair("response_type", "code");
            query.append_pair("client_id", &self.config.client_id);
            query.append_pair("redirect_uri", &self.config.redirect_uri);
            query.append_pair("scope", &self.config.scopes.join(" "));
            query.append_pair("state", &state);
            query.append_pair("code_challenge", &code_challenge);
            query.append_pair("code_challenge_method", "S256");
        }

        Ok(PkceAuthorization {
            url: url.to_string(),
            state,
            code_verifier,
        })
    }

    /// Exchanges an authorization code for tokens and stores them under `secret_ref`.
    pub async fn exchange_code(
        &self,
        secret_ref: &str,
        code: &str,
        code_verifier: &str,
    ) -> Result<OAuthTokenSet, ProviderError> {
        if code.trim().is_empty() || code_verifier.trim().is_empty() {
            return Err(ProviderError::OAuth {
                provider: OAUTH_PROVIDER.to_string(),
                message: "Authorization code and PKCE verifier must not be empty".to_string(),
            });
        }

        let redaction = SecretString::new(code.to_string()).ok();
        let form = [
            ("grant_type", "authorization_code"),
            ("client_id", self.config.client_id.as_str()),
            ("code", code),
            ("redirect_uri", self.config.redirect_uri.as_str()),
            ("code_verifier", code_verifier),
        ];

        let token = self.post_token_form(&form, redaction.as_ref()).await?;
        self.store_tokens(secret_ref, &token)?;
        Ok(token)
    }

    /// Refreshes the stored token set, preserving the existing refresh token when the provider
    /// omits a replacement.
    pub async fn refresh(&self, secret_ref: &str) -> Result<OAuthTokenSet, ProviderError> {
        let current = self.load_tokens(secret_ref)?;
        let refresh_token = current
            .refresh_token
            .clone()
            .ok_or_else(|| ProviderError::OAuth {
                provider: OAUTH_PROVIDER.to_string(),
                message: "Stored OAuth token set has no refresh token".to_string(),
            })?;

        let redaction = SecretString::new(refresh_token.clone()).ok();
        let form = [
            ("grant_type", "refresh_token"),
            ("client_id", self.config.client_id.as_str()),
            ("refresh_token", refresh_token.as_str()),
        ];

        let mut refreshed = self.post_token_form(&form, redaction.as_ref()).await?;
        if refreshed.refresh_token.is_none() {
            refreshed.refresh_token = Some(refresh_token);
        }
        self.store_tokens(secret_ref, &refreshed)?;
        Ok(refreshed)
    }

    pub fn load_tokens(&self, secret_ref: &str) -> Result<OAuthTokenSet, ProviderError> {
        let secret = self.store.get_secret(secret_ref)?;
        serde_json::from_str::<OAuthTokenSet>(secret.expose_secret()).map_err(|_| {
            ProviderError::OAuth {
                provider: OAUTH_PROVIDER.to_string(),
                message: "Stored credential is not a valid OAuth token set".to_string(),
            }
        })
    }

    pub fn store_tokens(
        &self,
        secret_ref: &str,
        tokens: &OAuthTokenSet,
    ) -> Result<(), ProviderError> {
        let serialized = serde_json::to_string(tokens).map_err(|e| ProviderError::OAuth {
            provider: OAUTH_PROVIDER.to_string(),
            message: format!("Failed to serialize OAuth token set: {e}"),
        })?;
        let secret = SecretString::new(serialized).map_err(|_| ProviderError::OAuth {
            provider: OAUTH_PROVIDER.to_string(),
            message: "OAuth token set serialized to an empty value".to_string(),
        })?;
        self.store.set_secret(secret_ref, secret)?;
        Ok(())
    }

    async fn post_token_form(
        &self,
        form: &[(&str, &str)],
        redaction: Option<&SecretString>,
    ) -> Result<OAuthTokenSet, ProviderError> {
        let response = self
            .client
            .post(&self.config.token_url)
            .form(form)
            .send()
            .await
            .map_err(|e| map_reqwest_send_error(OAUTH_PROVIDER, self.timeout_ms, &e, redaction))?;

        let status = response.status();
        let retry_after = retry_after_ms(response.headers());
        let body = response.text().await.unwrap_or_default();

        if !status.is_success() {
            return Err(map_provider_http_error(
                OAUTH_PROVIDER,
                status.as_u16(),
                &body,
                redaction,
                retry_after,
            ));
        }

        let token: TokenResponse =
            serde_json::from_str(&body).map_err(|e| ProviderError::MalformedResponse {
                provider: OAUTH_PROVIDER.to_string(),
                reason: format!("Invalid OAuth token response JSON: {e}"),
            })?;

        if token.access_token.trim().is_empty() {
            return Err(ProviderError::MalformedResponse {
                provider: OAUTH_PROVIDER.to_string(),
                reason: "OAuth token response is missing 'access_token'".to_string(),
            });
        }

        Ok(token.into_token_set())
    }
}

/// OpenAI-compatible adapter using a ChatGPT OAuth access token (or a plain API key fallback).
#[derive(Clone)]
pub struct ChatGptAdapter {
    provider_name: String,
    base_url: String,
    secret_ref: Option<String>,
    store: Arc<dyn CredentialStore>,
    oauth: Option<ChatGptOAuthClient>,
    client: reqwest::Client,
    timeout_ms: u64,
}

impl ChatGptAdapter {
    pub fn new(
        provider_name: impl Into<String>,
        base_url: &str,
        secret_ref: Option<String>,
        store: Arc<dyn CredentialStore>,
        oauth: Option<ChatGptOAuthClient>,
        timeout_ms: u64,
    ) -> Result<Self, ProviderError> {
        let provider_name = provider_name.into();
        let base_url = validate_provider_base_url(base_url)?;
        let client = reqwest::Client::builder()
            .timeout(Duration::from_millis(timeout_ms))
            .build()
            .map_err(|e| ProviderError::Transport {
                provider: provider_name.clone(),
                message: e.to_string(),
            })?;

        Ok(Self {
            provider_name,
            base_url,
            secret_ref,
            store,
            oauth,
            client,
            timeout_ms,
        })
    }

    /// Resolves a bearer token, refreshing an expiring OAuth token set when possible. A stored
    /// value that is not a serialized token set is treated as a plain API key.
    async fn access_token(&self) -> Result<SecretString, ProviderError> {
        let secret_ref =
            self.secret_ref
                .as_deref()
                .ok_or_else(|| ProviderError::AuthenticationFailed {
                    provider: self.provider_name.clone(),
                    message: "Missing credential reference for ChatGPT provider".to_string(),
                })?;

        let secret = self.store.get_secret(secret_ref)?;

        let Ok(tokens) = serde_json::from_str::<OAuthTokenSet>(secret.expose_secret()) else {
            return Ok(secret);
        };

        let expires_soon = tokens
            .expires_at_ms
            .map(|expires_at| expires_at.saturating_sub(now_unix_ms()) <= TOKEN_REFRESH_SKEW_MS)
            .unwrap_or(false);

        if expires_soon
            && let Some(oauth) = &self.oauth
            && tokens.refresh_token.is_some()
        {
            let refreshed = oauth.refresh(secret_ref).await?;
            return SecretString::new(refreshed.access_token).map_err(|_| {
                ProviderError::AuthenticationFailed {
                    provider: self.provider_name.clone(),
                    message: "Refreshed OAuth access token was empty".to_string(),
                }
            });
        }

        if expires_soon {
            return Err(ProviderError::AuthenticationFailed {
                provider: self.provider_name.clone(),
                message: "OAuth access token has expired and cannot be refreshed".to_string(),
            });
        }

        SecretString::new(tokens.access_token).map_err(|_| ProviderError::AuthenticationFailed {
            provider: self.provider_name.clone(),
            message: "Stored OAuth access token was empty".to_string(),
        })
    }
}

#[async_trait]
impl ModelProviderAdapter for ChatGptAdapter {
    fn provider_name(&self) -> &str {
        &self.provider_name
    }

    async fn complete(
        &self,
        req: &ModelCompletionRequest,
    ) -> Result<ModelCompletionResponse, ProviderError> {
        let token = self.access_token().await?;
        let endpoint = format!("{}/chat/completions", self.base_url);
        let payload = build_openai_chat_payload(req, false);

        let response = self
            .client
            .post(&endpoint)
            .bearer_auth(token.expose_secret())
            .json(&payload)
            .send()
            .await
            .map_err(|e| {
                map_reqwest_send_error(&self.provider_name, self.timeout_ms, &e, Some(&token))
            })?;

        let status = response.status();
        let retry_after = retry_after_ms(response.headers());
        let body = response.text().await.unwrap_or_default();

        if !status.is_success() {
            return Err(map_provider_http_error(
                &self.provider_name,
                status.as_u16(),
                &body,
                Some(&token),
                retry_after,
            ));
        }

        OpenAiCompatibleAdapter::parse_chat_completion_json(&self.provider_name, &body)
    }

    fn stream(&self, req: ModelCompletionRequest) -> Option<ModelCompletionStream> {
        let adapter = self.clone();

        Some(Box::pin(async_stream::try_stream! {
            let token = adapter.access_token().await?;
            let endpoint = format!("{}/chat/completions", adapter.base_url);
            let payload = build_openai_chat_payload(&req, true);

            let response = adapter
                .client
                .post(&endpoint)
                .bearer_auth(token.expose_secret())
                .json(&payload)
                .send()
                .await
                .map_err(|e| {
                    map_reqwest_send_error(
                        &adapter.provider_name,
                        adapter.timeout_ms,
                        &e,
                        Some(&token),
                    )
                })?;

            let response =
                ensure_success_response(&adapter.provider_name, response, Some(&token)).await?;

            let provider = adapter.provider_name.clone();
            let mut byte_stream = response.bytes_stream();
            let mut lines = LineAccumulator::new(&provider);
            let mut frames = SseFrameAccumulator::new(&provider);
            let mut state = OpenAiSseState::new();
            let mut finished = false;

            'stream: while let Some(chunk) = byte_stream.next().await {
                let chunk = chunk.map_err(|e| ProviderError::Transport {
                    provider: provider.clone(),
                    message: redact_sensitive_text(&e.to_string(), Some(&token)),
                })?;

                for line in lines.push(&chunk)? {
                    let Some(frame) = frames.push_line(&line)? else {
                        continue;
                    };
                    match state.process_data(&provider, &frame.data)? {
                        OpenAiChunkOutcome::Events(events) => {
                            for event in events {
                                yield event;
                            }
                        }
                        OpenAiChunkOutcome::Done(response) => {
                            yield ModelStreamEvent::Done(response);
                            finished = true;
                            break 'stream;
                        }
                    }
                }
            }

            if !finished {
                let mut trailing_frames = Vec::new();
                let trailing_frame = match lines.finish()? {
                    Some(line) => frames.push_line(&line)?,
                    None => None,
                };
                if let Some(frame) = trailing_frame {
                    trailing_frames.push(frame);
                }
                if let Some(frame) = frames.finish() {
                    trailing_frames.push(frame);
                }

                for frame in trailing_frames {
                    match state.process_data(&provider, &frame.data)? {
                        OpenAiChunkOutcome::Events(events) => {
                            for event in events {
                                yield event;
                            }
                        }
                        OpenAiChunkOutcome::Done(response) => {
                            yield ModelStreamEvent::Done(response);
                            finished = true;
                            break;
                        }
                    }
                }
            }

            if !finished {
                yield ModelStreamEvent::Done(state.snapshot(&provider)?);
            }
        }))
    }
}

/// Builds an authorization URL using S256 PKCE, a random `state`, and the configured scopes.
fn pkce_s256_challenge(verifier: &str) -> String {
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(verifier.as_bytes());
    base64_url_no_pad(&digest)
}

fn random_urlsafe(byte_len: usize) -> String {
    let mut material = Vec::with_capacity(byte_len);
    while material.len() < byte_len {
        material.extend_from_slice(uuid::Uuid::new_v4().as_bytes());
    }
    material.truncate(byte_len);
    base64_url_no_pad(&material)
}

fn base64_url_no_pad(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

    let mut output = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let first = chunk[0] as u32;
        let second = chunk.get(1).copied().unwrap_or(0) as u32;
        let third = chunk.get(2).copied().unwrap_or(0) as u32;
        let combined = (first << 16) | (second << 8) | third;

        output.push(ALPHABET[((combined >> 18) & 0x3f) as usize] as char);
        output.push(ALPHABET[((combined >> 12) & 0x3f) as usize] as char);
        if chunk.len() > 1 {
            output.push(ALPHABET[((combined >> 6) & 0x3f) as usize] as char);
        }
        if chunk.len() > 2 {
            output.push(ALPHABET[(combined & 0x3f) as usize] as char);
        }
    }
    output
}
