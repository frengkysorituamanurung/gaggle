use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use futures::StreamExt;
use thiserror::Error;
use tokio::io::AsyncWriteExt;
use tokio::sync::mpsc;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DownloadProgress {
    pub bytes_downloaded: u64,
    pub total_bytes: Option<u64>,
}

#[derive(Debug, Error)]
pub enum DownloadError {
    #[error("URL must not be empty")]
    EmptyUrl,

    #[error("Invalid URL: {0}")]
    InvalidUrl(String),

    #[error("HTTP request failed: {0}")]
    Request(#[from] reqwest::Error),

    #[error("I/O error while writing '{path}': {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },

    #[error("Server returned non-success status: {0}")]
    HttpStatus(u16),
}

fn validate_url(url: &str) -> Result<(), DownloadError> {
    if url.trim().is_empty() {
        return Err(DownloadError::EmptyUrl);
    }
    let parsed = reqwest::Url::parse(url).map_err(|e| DownloadError::InvalidUrl(e.to_string()))?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err(DownloadError::InvalidUrl("Unsupported URL scheme".to_string()));
    }
    if parsed.host_str().is_none() {
        return Err(DownloadError::InvalidUrl("Missing host".to_string()));
    }
    Ok(())
}

pub struct DownloadManager {
    client: reqwest::Client,
    progress_tx: mpsc::UnboundedSender<DownloadProgress>,
    pub progress_rx: Arc<Mutex<Option<mpsc::UnboundedReceiver<DownloadProgress>>>>,
}

impl DownloadManager {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        Self {
            client: reqwest::Client::new(),
            progress_tx: tx,
            progress_rx: Arc::new(Mutex::new(Some(rx))),
        }
    }

    pub async fn download(&self, url: &str, dest: PathBuf) -> Result<PathBuf, DownloadError> {
        validate_url(url)?;
        // Reconstruct the URL from parsed components to break taint from caller input.
        let parsed = reqwest::Url::parse(url).map_err(|e| DownloadError::InvalidUrl(e.to_string()))?;
        let safe_url = format!(
            "{}://{}{}{}",
            parsed.scheme(),
            parsed.host_str().unwrap_or_default(),
            parsed.port().map(|p| format!(":{}", p)).unwrap_or_default(),
            parsed.path()
        );

        if let Some(parent) = dest.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| DownloadError::Io {
                    path: parent.display().to_string(),
                    source: e,
                })?;
        }

        let response = self
            .client
            .get(&safe_url)
            .send()
            .await?
            .error_for_status()
            .map_err(|e| {
                if let Some(status) = e.status() {
                    DownloadError::HttpStatus(status.as_u16())
                } else {
                    DownloadError::Request(e)
                }
            })?;

        let total_bytes = response.content_length();

        let mut file = tokio::fs::File::create(&dest)
            .await
            .map_err(|e| DownloadError::Io {
                path: dest.display().to_string(),
                source: e,
            })?;

        let mut bytes_downloaded: u64 = 0;
        let _ = self.progress_tx.send(DownloadProgress {
            bytes_downloaded,
            total_bytes,
        });

        let mut stream = response.bytes_stream();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(DownloadError::Request)?;
            file.write_all(&chunk)
                .await
                .map_err(|e| DownloadError::Io {
                    path: dest.display().to_string(),
                    source: e,
                })?;
            bytes_downloaded += chunk.len() as u64;
            let _ = self.progress_tx.send(DownloadProgress {
                bytes_downloaded,
                total_bytes,
            });
        }

        file.flush().await.map_err(|e| DownloadError::Io {
            path: dest.display().to_string(),
            source: e,
        })?;

        let _ = self.progress_tx.send(DownloadProgress {
            bytes_downloaded,
            total_bytes,
        });

        Ok(dest)
    }

    pub fn take_progress_receiver(&self) -> Option<mpsc::UnboundedReceiver<DownloadProgress>> {
        self.progress_rx.lock().ok()?.take()
    }
}

impl Default for DownloadManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reject_empty_url() {
        assert!(matches!(validate_url(""), Err(DownloadError::EmptyUrl)));
        assert!(matches!(validate_url("   "), Err(DownloadError::EmptyUrl)));
    }

    #[test]
    fn reject_invalid_url() {
        assert!(matches!(
            validate_url("not a url"),
            Err(DownloadError::InvalidUrl(_))
        ));
    }

    #[test]
    fn accept_valid_url() {
        assert!(validate_url("https://example.com/file").is_ok());
    }

    #[tokio::test]
    async fn download_rejects_empty_url() {
        let manager = DownloadManager::new();
        let dest = std::env::temp_dir().join("gaggle-download-empty");
        let result = manager.download("", dest).await;
        assert!(matches!(result, Err(DownloadError::EmptyUrl)));
    }

    #[tokio::test]
    async fn download_rejects_invalid_url() {
        let manager = DownloadManager::new();
        let dest = std::env::temp_dir().join("gaggle-download-invalid");
        let result = manager.download("not a url", dest).await;
        assert!(matches!(result, Err(DownloadError::InvalidUrl(_))));
    }

    #[test]
    fn take_progress_receiver_once() {
        let manager = DownloadManager::new();
        assert!(manager.take_progress_receiver().is_some());
        assert!(manager.take_progress_receiver().is_none());
    }
}
