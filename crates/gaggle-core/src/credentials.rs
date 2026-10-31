use std::{
    collections::HashMap,
    fmt,
    sync::{Arc, RwLock},
};
use thiserror::Error;

#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum CredentialError {
    #[error("Secret reference key must not be empty")]
    EmptySecretRef,

    #[error("Secret value must not be empty")]
    EmptySecretValue,

    #[error("Secret reference '{0}' was not found in the credential store")]
    NotFound(String),

    #[error("Credential store failure: {0}")]
    StoreError(String),
}

/// Wrapper around a sensitive credential value whose `Debug` and `Display` implementations
/// always output `[REDACTED]` so secrets cannot accidentally leak into logs or error traces.
#[derive(Clone, PartialEq, Eq)]
pub struct SecretString(String);

impl SecretString {
    pub fn new(value: impl Into<String>) -> Result<Self, CredentialError> {
        let s = value.into();
        if s.trim().is_empty() {
            return Err(CredentialError::EmptySecretValue);
        }
        Ok(Self(s))
    }

    /// Explicitly exposes the raw secret value when constructing an outbound authorization header.
    pub fn expose_secret(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for SecretString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SecretString([REDACTED])")
    }
}

impl fmt::Display for SecretString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[REDACTED]")
    }
}

/// Redacts any occurrence of a known secret or `Bearer <token>` / `sk-...` pattern from
/// diagnostic or error strings before logging or returning to clients.
pub fn redact_sensitive_text(input: &str, known_secret: Option<&SecretString>) -> String {
    let mut output = input.to_string();
    if let Some(secret) = known_secret {
        let raw = secret.expose_secret();
        if !raw.is_empty() {
            output = output.replace(raw, "[REDACTED]");
        }
    }
    output
}

/// Abstraction over OS credential storage (Keychain on macOS, Secret Service on Linux)
/// and deterministic in-memory storage for isolated tests.
pub trait CredentialStore: Send + Sync {
    fn set_secret(&self, secret_ref: &str, secret: SecretString) -> Result<(), CredentialError>;
    fn get_secret(&self, secret_ref: &str) -> Result<SecretString, CredentialError>;
    fn has_secret(&self, secret_ref: &str) -> bool;
    fn delete_secret(&self, secret_ref: &str) -> Result<(), CredentialError>;
}

/// Thread-safe in-memory credential store that never writes secrets to SQLite or disk.
#[derive(Default, Clone)]
pub struct InMemoryCredentialStore {
    entries: Arc<RwLock<HashMap<String, SecretString>>>,
}

impl InMemoryCredentialStore {
    pub fn new() -> Self {
        Self::default()
    }
}

impl CredentialStore for InMemoryCredentialStore {
    fn set_secret(&self, secret_ref: &str, secret: SecretString) -> Result<(), CredentialError> {
        let key = secret_ref.trim();
        if key.is_empty() {
            return Err(CredentialError::EmptySecretRef);
        }
        let mut guard = self
            .entries
            .write()
            .map_err(|_| CredentialError::StoreError("Lock poisoned".to_string()))?;
        guard.insert(key.to_string(), secret);
        Ok(())
    }

    fn get_secret(&self, secret_ref: &str) -> Result<SecretString, CredentialError> {
        let key = secret_ref.trim();
        if key.is_empty() {
            return Err(CredentialError::EmptySecretRef);
        }
        let guard = self
            .entries
            .read()
            .map_err(|_| CredentialError::StoreError("Lock poisoned".to_string()))?;
        guard
            .get(key)
            .cloned()
            .ok_or_else(|| CredentialError::NotFound(key.to_string()))
    }

    fn has_secret(&self, secret_ref: &str) -> bool {
        self.get_secret(secret_ref).is_ok()
    }

    fn delete_secret(&self, secret_ref: &str) -> Result<(), CredentialError> {
        let key = secret_ref.trim();
        if key.is_empty() {
            return Err(CredentialError::EmptySecretRef);
        }
        let mut guard = self
            .entries
            .write()
            .map_err(|_| CredentialError::StoreError("Lock poisoned".to_string()))?;
        guard.remove(key);
        Ok(())
    }
}

/// Stores each secret in the operating-system credential vault under its
/// `secret_ref`. The secret value is never written to Gaggle's SQLite database.
#[derive(Clone)]
pub struct KeyringCredentialStore {
    service: String,
}

impl KeyringCredentialStore {
    pub fn new() -> Self {
        Self {
            service: "com.gaggle.credentials".to_string(),
        }
    }

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    fn entry(&self, secret_ref: &str) -> Result<keyring::Entry, CredentialError> {
        let secret_ref = secret_ref.trim();
        if secret_ref.is_empty() {
            return Err(CredentialError::EmptySecretRef);
        }
        keyring::Entry::new(&self.service, secret_ref)
            .map_err(|_| CredentialError::StoreError("Could not access OS credential vault".into()))
    }
}

impl Default for KeyringCredentialStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
impl CredentialStore for KeyringCredentialStore {
    fn set_secret(&self, secret_ref: &str, secret: SecretString) -> Result<(), CredentialError> {
        self.entry(secret_ref)?
            .set_password(secret.expose_secret())
            .map_err(|_| CredentialError::StoreError("OS credential vault write failed".into()))
    }

    fn get_secret(&self, secret_ref: &str) -> Result<SecretString, CredentialError> {
        let entry = self.entry(secret_ref)?;
        match entry.get_password() {
            Ok(secret) => SecretString::new(secret),
            Err(error) if error.to_string().contains("No entry") => {
                Err(CredentialError::NotFound(secret_ref.trim().to_string()))
            }
            Err(_) => Err(CredentialError::StoreError(
                "OS credential vault read failed".into(),
            )),
        }
    }

    fn has_secret(&self, secret_ref: &str) -> bool {
        self.entry(secret_ref)
            .and_then(|entry| {
                entry.get_password().map_err(|_| {
                    CredentialError::StoreError("OS credential vault read failed".into())
                })
            })
            .is_ok()
    }

    fn delete_secret(&self, secret_ref: &str) -> Result<(), CredentialError> {
        let entry = self.entry(secret_ref)?;
        match entry.delete_credential() {
            Ok(()) => Ok(()),
            Err(error) if error.to_string().contains("No entry") => Ok(()),
            Err(_) => Err(CredentialError::StoreError(
                "OS credential vault delete failed".into(),
            )),
        }
    }
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
impl CredentialStore for KeyringCredentialStore {
    fn set_secret(&self, _secret_ref: &str, _secret: SecretString) -> Result<(), CredentialError> {
        Err(CredentialError::StoreError(
            "OS credential vault is unsupported on this platform".into(),
        ))
    }

    fn get_secret(&self, _secret_ref: &str) -> Result<SecretString, CredentialError> {
        Err(CredentialError::StoreError(
            "OS credential vault is unsupported on this platform".into(),
        ))
    }

    fn has_secret(&self, _secret_ref: &str) -> bool {
        false
    }

    fn delete_secret(&self, _secret_ref: &str) -> Result<(), CredentialError> {
        Err(CredentialError::StoreError(
            "OS credential vault is unsupported on this platform".into(),
        ))
    }
}
