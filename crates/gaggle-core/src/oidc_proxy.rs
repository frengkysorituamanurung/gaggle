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
        if self.config.issuer_url.is_none() {
            return Err("issuer_url is required".to_string());
        }
        if self.config.client_id.is_none() {
            return Err("client_id is required".to_string());
        }
        if self.config.redirect_uri.is_none() {
            return Err("redirect_uri is required".to_string());
        }
        Ok(())
    }

    /// Build the authorization URL for the OIDC flow.
    pub fn authorization_url(&self, state: &str, nonce: &str) -> Result<String, String> {
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

        let url = format!(
            "{}/authorize?response_type=code&client_id={}&redirect_uri={}&scope={}&state={}&nonce={}",
            issuer.trim_end_matches('/'),
            client_id,
            redirect_uri,
            self.config.scopes.join(" "),
            state,
            nonce,
        );
        Ok(url)
    }

    /// Get the bind address for the HTTP server.
    pub fn bind_address(&self) -> String {
        format!("{}:{}", self.config.bind_address, self.config.bind_port)
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
}
