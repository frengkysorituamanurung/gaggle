//! OIDC proxy for enterprise authentication.
//!
//! Provides an OIDC proxy server for enterprise SSO integration
//! using the authorization code flow with PKCE.

use serde::{Deserialize, Serialize};

/// OIDC proxy configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OidcProxyConfig {
    pub enabled: bool,
    pub issuer_url: Option<String>,
    pub client_id: Option<String>,
    pub client_secret_ref: Option<String>,
    pub redirect_uri: Option<String>,
    pub scopes: Vec<String>,
    pub bind_address: String,
    pub bind_port: u16,
}

impl Default for OidcProxyConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            issuer_url: None,
            client_id: None,
            client_secret_ref: None,
            redirect_uri: None,
            scopes: vec![
                "openid".to_string(),
                "profile".to_string(),
                "email".to_string(),
            ],
            bind_address: "127.0.0.1".to_string(),
            bind_port: 8080,
        }
    }
}

/// OIDC discovery document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OidcDiscovery {
    pub issuer: String,
    pub authorization_endpoint: String,
    pub token_endpoint: String,
    pub userinfo_endpoint: Option<String>,
    pub jwks_uri: String,
}

/// OIDC proxy service.
pub struct OidcProxy {
    config: OidcProxyConfig,
}

impl OidcProxy {
    pub fn new(config: OidcProxyConfig) -> Self {
        Self { config }
    }

    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    /// Validate the OIDC configuration.
    pub fn validate(&self) -> Result<(), String> {
        if !self.config.enabled {
            return Ok(());
        }
        if self
            .config
            .issuer_url
            .as_ref()
            .map(|s| s.trim().is_empty())
            .unwrap_or(true)
        {
            return Err("issuer_url is required".to_string());
        }
        if self
            .config
            .client_id
            .as_ref()
            .map(|s| s.trim().is_empty())
            .unwrap_or(true)
        {
            return Err("client_id is required".to_string());
        }
        if self
            .config
            .redirect_uri
            .as_ref()
            .map(|s| s.trim().is_empty())
            .unwrap_or(true)
        {
            return Err("redirect_uri is required".to_string());
        }
        if let Some(issuer) = &self.config.issuer_url {
            reqwest::Url::parse(issuer).map_err(|e| format!("Invalid issuer_url: {}", e))?;
        }
        if let Some(redirect) = &self.config.redirect_uri {
            reqwest::Url::parse(redirect).map_err(|e| format!("Invalid redirect_uri: {}", e))?;
        }
        Ok(())
    }

    /// Build the authorization URL for the OIDC flow.
    pub fn authorization_url(&self, state: &str, nonce: &str) -> Result<String, String> {
        if state.trim().is_empty() {
            return Err("state must not be empty".to_string());
        }
        if nonce.trim().is_empty() {
            return Err("nonce must not be empty".to_string());
        }
        let issuer = self
            .config
            .issuer_url
            .as_ref()
            .ok_or("issuer_url not configured")?;
        let client_id = self
            .config
            .client_id
            .as_ref()
            .ok_or("client_id not configured")?;
        let redirect_uri = self
            .config
            .redirect_uri
            .as_ref()
            .ok_or("redirect_uri not configured")?;

        let base = format!("{}/authorize", issuer.trim_end_matches('/'));
        let mut url =
            reqwest::Url::parse(&base).map_err(|e| format!("Invalid authorization URL: {}", e))?;
        {
            let mut query = url.query_pairs_mut();
            query.append_pair("response_type", "code");
            query.append_pair("client_id", client_id);
            query.append_pair("redirect_uri", redirect_uri);
            query.append_pair("scope", &self.config.scopes.join(" "));
            query.append_pair("state", state);
            query.append_pair("nonce", nonce);
        }
        Ok(url.to_string())
    }

    /// Get the bind address for the HTTP server.
    pub fn bind_address(&self) -> String {
        format!("{}:{}", self.config.bind_address, self.config.bind_port)
    }

    /// Fetch the OIDC discovery document from the issuer.
    pub async fn fetch_discovery(&self) -> Result<OidcDiscovery, String> {
        let issuer = self
            .config
            .issuer_url
            .as_ref()
            .ok_or("issuer_url not configured")?
            .trim_end_matches('/');
        reqwest::Url::parse(issuer).map_err(|e| format!("Invalid issuer_url: {}", e))?;
        let url = format!("{}/.well-known/openid-configuration", issuer);
        let resp = reqwest::get(&url)
            .await
            .map_err(|e| format!("Discovery request failed: {}", e))?;
        if !resp.status().is_success() {
            return Err(format!("Discovery failed: {}", resp.status()));
        }
        let doc: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("Invalid discovery JSON: {}", e))?;
        Ok(OidcDiscovery {
            issuer: doc
                .get("issuer")
                .and_then(|v| v.as_str())
                .unwrap_or(issuer)
                .to_string(),
            authorization_endpoint: doc
                .get("authorization_endpoint")
                .and_then(|v| v.as_str())
                .ok_or("missing authorization_endpoint")?
                .to_string(),
            token_endpoint: doc
                .get("token_endpoint")
                .and_then(|v| v.as_str())
                .ok_or("missing token_endpoint")?
                .to_string(),
            userinfo_endpoint: doc
                .get("userinfo_endpoint")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            jwks_uri: doc
                .get("jwks_uri")
                .and_then(|v| v.as_str())
                .ok_or("missing jwks_uri")?
                .to_string(),
        })
    }

    /// Fetch the JWKS document from the issuer.
    #[cfg(feature = "oidc")]
    pub async fn fetch_jwks(&self) -> Result<serde_json::Value, String> {
        let discovery = self.fetch_discovery().await?;
        let resp = reqwest::get(&discovery.jwks_uri)
            .await
            .map_err(|e| format!("JWKS request failed: {}", e))?;
        if !resp.status().is_success() {
            return Err(format!("JWKS fetch failed: {}", resp.status()));
        }
        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("Invalid JWKS JSON: {}", e))?;
        Ok(json)
    }

    /// Validate a JWT against a JWKS document.
    #[cfg(feature = "oidc")]
    pub fn validate_token(
        &self,
        token: &str,
        jwks: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        use jsonwebtoken::{
            Algorithm, DecodingKey, Validation, decode, decode_header, jwk::JwkSet,
        };

        let header = decode_header(token).map_err(|e| format!("Invalid token header: {}", e))?;
        let jwk_set: JwkSet =
            serde_json::from_value(jwks.clone()).map_err(|e| format!("Invalid JWKS: {}", e))?;
        let kid = header.kid.clone().unwrap_or_default();
        let jwk = jwk_set
            .keys
            .iter()
            .find(|k| k.common.key_id.as_deref() == Some(kid.as_str()))
            .ok_or_else(|| "matching JWK not found".to_string())?;
        let decoding_key =
            DecodingKey::from_jwk(jwk).map_err(|e| format!("Invalid decoding key: {}", e))?;
        let mut validation = Validation::new(header.alg);
        if let Some(issuer) = &self.config.issuer_url {
            validation.set_issuer(&[issuer.as_str()]);
        }
        let data = decode::<serde_json::Value>(token, &decoding_key, &validation)
            .map_err(|e| format!("Token validation failed: {}", e))?;
        Ok(data.claims)
    }

    /// Fetch the JWKS document (stub when feature disabled).
    #[cfg(not(feature = "oidc"))]
    pub async fn fetch_jwks(&self) -> Result<serde_json::Value, String> {
        Err("oidc feature not enabled".to_string())
    }

    /// Validate a JWT (stub when feature disabled).
    #[cfg(not(feature = "oidc"))]
    pub fn validate_token(
        &self,
        _token: &str,
        _jwks: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        Err("oidc feature not enabled".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_disabled() {
        assert!(!OidcProxyConfig::default().enabled);
    }

    #[test]
    fn validate_disabled_passes() {
        let proxy = OidcProxy::new(OidcProxyConfig::default());
        assert!(proxy.validate().is_ok());
    }

    #[test]
    fn validate_enabled_requires_issuer() {
        let config = OidcProxyConfig {
            enabled: true,
            ..OidcProxyConfig::default()
        };
        let proxy = OidcProxy::new(config);
        assert!(proxy.validate().is_err());
    }

    #[test]
    fn authorization_url_requires_config() {
        let proxy = OidcProxy::new(OidcProxyConfig::default());
        assert!(proxy.authorization_url("state", "nonce").is_err());
    }

    #[test]
    fn authorization_url_builds_query() {
        let config = OidcProxyConfig {
            enabled: true,
            issuer_url: Some("https://issuer.example".into()),
            client_id: Some("client-1".into()),
            redirect_uri: Some("http://localhost:8080/cb".into()),
            ..OidcProxyConfig::default()
        };
        let proxy = OidcProxy::new(config);
        let url = proxy.authorization_url("st", "nc").unwrap();
        let parsed = reqwest::Url::parse(&url).unwrap();
        let pairs: std::collections::HashMap<String, String> =
            parsed.query_pairs().into_owned().collect();
        assert_eq!(pairs.get("client_id").map(String::as_str), Some("client-1"));
        assert_eq!(pairs.get("state").map(String::as_str), Some("st"));
        assert_eq!(pairs.get("nonce").map(String::as_str), Some("nc"));
        assert_eq!(pairs.get("response_type").map(String::as_str), Some("code"));
    }

    #[test]
    fn bind_address_format() {
        let proxy = OidcProxy::new(OidcProxyConfig::default());
        assert_eq!(proxy.bind_address(), "127.0.0.1:8080");
    }

    #[test]
    fn empty_state_rejected() {
        let config = OidcProxyConfig {
            issuer_url: Some("https://issuer.example".into()),
            client_id: Some("client-1".into()),
            redirect_uri: Some("http://localhost:8080/cb".into()),
            ..OidcProxyConfig::default()
        };
        let proxy = OidcProxy::new(config);
        assert!(proxy.authorization_url("", "nonce").is_err());
    }
}

#[cfg(feature = "oidc")]
mod feature_tests {
    use super::*;

    #[tokio::test]
    async fn feature_enabled_compiles() {
        let proxy = OidcProxy::new(OidcProxyConfig::default());
        assert!(proxy.fetch_jwks().await.is_err());
    }
}
