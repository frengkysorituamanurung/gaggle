use anyhow::{Context, Result};
use async_trait::async_trait;
use gaggle_db::{models::Extension, repositories::extensions::ExtensionRepository};
use sqlx::SqlitePool;
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::sync::Mutex;

use crate::McpClient;

// ---------------------------------------------------------------------------
// McpConnectionManager
// ---------------------------------------------------------------------------

/// Reusable connection cache for MCP extensions.
///
/// `McpConnectionManager` keeps one (initialized) `McpClient` per extension id
/// and reuses it across discovery and tool execution, avoiding connect-per-call.
#[derive(Clone)]
pub struct McpConnectionManager {
    pool: Option<SqlitePool>,
    clients: Arc<Mutex<HashMap<String, Arc<McpClient>>>>,
}

impl Default for McpConnectionManager {
    fn default() -> Self {
        Self {
            pool: None,
            clients: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

impl McpConnectionManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_pool(pool: SqlitePool) -> Self {
        Self {
            pool: Some(pool),
            clients: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn new_with_pool(pool: SqlitePool) -> Self {
        Self::with_pool(pool)
    }

    pub fn pool(&self) -> Option<SqlitePool> {
        self.pool.clone()
    }

    /// Return a cached, initialized client for `target`, connecting if needed.
    pub async fn get_or_connect<T>(&self, target: T) -> Result<Arc<McpClient>>
    where
        T: ConnectTarget,
    {
        let ext = target
            .resolve(&self.pool)
            .await
            .map_err(|e| anyhow::anyhow!(e))?;

        // Fast path: cached client.
        {
            let guard = self.clients.lock().await;
            if let Some(client) = guard.get(&ext.id) {
                return Ok(client.clone());
            }
        }

        // Connect + initialize outside the lock.
        let client = McpClient::from_extension(&ext, None)
            .await
            .context("Failed to connect MCP client")?;
        client
            .initialize()
            .await
            .context("Failed to initialize MCP client")?;
        let client = Arc::new(client);

        // Re-check under lock to avoid races.
        let mut guard = self.clients.lock().await;
        if let Some(existing) = guard.get(&ext.id) {
            // Another task connected first; close the duplicate.
            let _ = client.close().await;
            return Ok(existing.clone());
        }
        guard.insert(ext.id.clone(), client.clone());
        Ok(client)
    }

    /// Check whether the connection for `target` is healthy.
    pub async fn health_check<T>(&self, target: T) -> bool
    where
        T: HealthTarget,
    {
        target.health(self).await
    }

    /// Remove and close the cached connection for `target`.
    pub async fn invalidate<T>(&self, target: T) -> Result<()>
    where
        T: InvalidateTarget,
    {
        target.invalidate(self).await
    }

    /// Close every cached connection.
    pub async fn shutdown_all(&self) -> Result<()> {
        let clients: Vec<Arc<McpClient>> = {
            let mut guard = self.clients.lock().await;
            let values = guard.values().cloned().collect();
            guard.clear();
            values
        };
        for client in clients {
            let _ = client.close().await;
        }
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Internal helpers
    // -----------------------------------------------------------------------

    pub(crate) async fn ping_client(client: &McpClient) -> bool {
        let transport = client.transport();
        match tokio::time::timeout(
            Duration::from_secs(5),
            transport.request("ping", Some(serde_json::json!({}))),
        )
        .await
        {
            Ok(Ok(_)) => true,
            Ok(Err(_)) => client.list_tools().await.is_ok(),
            Err(_) => false,
        }
    }

    pub(crate) async fn client_for_id(&self, id: &str) -> Option<Arc<McpClient>> {
        let guard = self.clients.lock().await;
        guard.get(id).cloned()
    }

    pub(crate) async fn remove_id(&self, id: &str) -> Option<Arc<McpClient>> {
        let mut guard = self.clients.lock().await;
        guard.remove(id)
    }
}

// ---------------------------------------------------------------------------
// Traits
// ---------------------------------------------------------------------------

#[async_trait]
pub trait ConnectTarget {
    async fn resolve(&self, pool: &Option<SqlitePool>) -> Result<Extension, String>;
}

#[async_trait]
impl ConnectTarget for Extension {
    async fn resolve(&self, _pool: &Option<SqlitePool>) -> Result<Extension, String> {
        Ok(self.clone())
    }
}

#[async_trait]
impl ConnectTarget for &Extension {
    async fn resolve(&self, _pool: &Option<SqlitePool>) -> Result<Extension, String> {
        Ok((*self).clone())
    }
}

#[async_trait]
impl ConnectTarget for &str {
    async fn resolve(&self, pool: &Option<SqlitePool>) -> Result<Extension, String> {
        let pool = pool
            .clone()
            .ok_or_else(|| "McpConnectionManager has no pool".to_string())?;
        let repo = ExtensionRepository::new(pool);
        repo.get_extension_by_id(self)
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("Extension {self} not found"))
    }
}

#[async_trait]
impl ConnectTarget for String {
    async fn resolve(&self, pool: &Option<SqlitePool>) -> Result<Extension, String> {
        self.as_str().resolve(pool).await
    }
}

#[async_trait]
impl ConnectTarget for &String {
    async fn resolve(&self, pool: &Option<SqlitePool>) -> Result<Extension, String> {
        self.as_str().resolve(pool).await
    }
}

#[async_trait]
pub trait HealthTarget {
    async fn health(&self, manager: &McpConnectionManager) -> bool;
}

#[async_trait]
impl HealthTarget for &str {
    async fn health(&self, manager: &McpConnectionManager) -> bool {
        let id = (*self).to_string();
        let client = match manager.client_for_id(&id).await {
            Some(c) => c,
            None => return false,
        };
        let healthy = McpConnectionManager::ping_client(&client).await;
        if !healthy && let Some(c) = manager.remove_id(&id).await {
            let _ = c.close().await;
        }
        healthy
    }
}

#[async_trait]
impl HealthTarget for String {
    async fn health(&self, manager: &McpConnectionManager) -> bool {
        self.as_str().health(manager).await
    }
}

#[async_trait]
impl HealthTarget for &String {
    async fn health(&self, manager: &McpConnectionManager) -> bool {
        self.as_str().health(manager).await
    }
}

#[async_trait]
impl HealthTarget for Extension {
    async fn health(&self, manager: &McpConnectionManager) -> bool {
        self.id.as_str().health(manager).await
    }
}

#[async_trait]
impl HealthTarget for &Extension {
    async fn health(&self, manager: &McpConnectionManager) -> bool {
        self.id.as_str().health(manager).await
    }
}

#[async_trait]
impl HealthTarget for Arc<McpClient> {
    async fn health(&self, _manager: &McpConnectionManager) -> bool {
        McpConnectionManager::ping_client(self).await
    }
}

#[async_trait]
impl HealthTarget for &Arc<McpClient> {
    async fn health(&self, _manager: &McpConnectionManager) -> bool {
        McpConnectionManager::ping_client(*self).await
    }
}

#[async_trait]
impl HealthTarget for &McpClient {
    async fn health(&self, _manager: &McpConnectionManager) -> bool {
        McpConnectionManager::ping_client(*self).await
    }
}

#[async_trait]
pub trait InvalidateTarget {
    async fn invalidate(&self, manager: &McpConnectionManager) -> Result<()>;
}

#[async_trait]
impl InvalidateTarget for &str {
    async fn invalidate(&self, manager: &McpConnectionManager) -> Result<()> {
        let id = (*self).to_string();
        if let Some(client) = manager.remove_id(&id).await {
            let _ = client.close().await;
        }
        Ok(())
    }
}

#[async_trait]
impl InvalidateTarget for String {
    async fn invalidate(&self, manager: &McpConnectionManager) -> Result<()> {
        self.as_str().invalidate(manager).await
    }
}

#[async_trait]
impl InvalidateTarget for &String {
    async fn invalidate(&self, manager: &McpConnectionManager) -> Result<()> {
        self.as_str().invalidate(manager).await
    }
}

#[async_trait]
impl InvalidateTarget for Extension {
    async fn invalidate(&self, manager: &McpConnectionManager) -> Result<()> {
        self.id.as_str().invalidate(manager).await
    }
}

#[async_trait]
impl InvalidateTarget for &Extension {
    async fn invalidate(&self, manager: &McpConnectionManager) -> Result<()> {
        self.id.as_str().invalidate(manager).await
    }
}

#[async_trait]
impl InvalidateTarget for Arc<McpClient> {
    async fn invalidate(&self, manager: &McpConnectionManager) -> Result<()> {
        // Find by pointer.
        let mut guard = manager.clients.lock().await;
        let found = guard
            .iter()
            .find(|(_, v)| Arc::ptr_eq(v, self))
            .map(|(k, _)| k.clone());
        if let Some(id) = found
            && let Some(c) = guard.remove(&id)
        {
            drop(guard);
            let _ = c.close().await;
            return Ok(());
        }
        drop(guard);
        let _ = self.close().await;
        Ok(())
    }
}

#[async_trait]
impl InvalidateTarget for &Arc<McpClient> {
    async fn invalidate(&self, manager: &McpConnectionManager) -> Result<()> {
        (**self).invalidate(manager).await
    }
}

#[async_trait]
impl InvalidateTarget for &McpClient {
    async fn invalidate(&self, manager: &McpConnectionManager) -> Result<()> {
        // Cannot map to cached id reliably; just attempt close.
        // (No-op for cache.)
        let _ = manager;
        Ok(())
    }
}
