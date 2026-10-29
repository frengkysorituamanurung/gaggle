//! OIDC proxy for enterprise authentication.
//!
//! Provides an OIDC proxy server for enterprise SSO integration.

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
        }
    }
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_disabled() {
        assert!(!OidcProxyConfig::default().enabled);
    }
}
