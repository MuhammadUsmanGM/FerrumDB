use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::sync::RwLock;
use tracing::{info, warn};

use crate::error::FerrumError;

/// Core key-value storage engine with in-memory HashMap and file-based persistence.
pub struct StorageEngine {
    data: RwLock<HashMap<String, String>>,
    path: PathBuf,
}

impl StorageEngine {
    /// Create a new storage engine, loading existing data from disk if available.
    pub async fn new(path: impl AsRef<Path>) -> Result<Self, FerrumError> {
        let path = path.as_ref().to_path_buf();
        let data = if path.exists() {
            let contents = fs::read_to_string(&path).await.map_err(FerrumError::Io)?;
            serde_json::from_str(&contents).unwrap_or_else(|e| {
                warn!("Failed to parse storage file, starting fresh: {e}");
                HashMap::new()
            })
        } else {
            HashMap::new()
        };
        let count = data.len();
        info!("Storage engine initialized with {count} entries from {}", path.display());
        Ok(Self {
            data: RwLock::new(data),
            path,
        })
    }

    pub async fn get(&self, key: &str) -> Option<String> {
        let data = self.data.read().await;
        data.get(key).cloned()
    }

    pub async fn set(&self, key: String, value: String) -> Result<Option<String>, FerrumError> {
        let old = {
            let mut data = self.data.write().await;
            data.insert(key.clone(), value.clone())
        };
        self.persist().await?;
        info!("SET {key}");
        Ok(old)
    }

    pub async fn delete(&self, key: &str) -> Result<Option<String>, FerrumError> {
        let old = {
            let mut data = self.data.write().await;
            data.remove(key)
        };
        if old.is_some() {
            self.persist().await?;
            info!("DELETE {key}");
        }
        Ok(old)
    }

    pub async fn keys(&self) -> Vec<String> {
        let data = self.data.read().await;
        data.keys().cloned().collect()
    }

    pub async fn len(&self) -> usize {
        let data = self.data.read().await;
        data.len()
    }

    async fn persist(&self) -> Result<(), FerrumError> {
        let data = self.data.read().await;
        let json = serde_json::to_string_pretty(&*data).map_err(FerrumError::Serialize)?;
        fs::write(&self.path, json).await.map_err(FerrumError::Io)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_set_get_delete() {
        let tmp = NamedTempFile::new().unwrap();
        let engine = StorageEngine::new(tmp.path()).await.unwrap();

        assert!(engine.get("key").await.is_none());

        engine.set("key".into(), "value".into()).await.unwrap();
        assert_eq!(engine.get("key").await.unwrap(), "value");

        engine.delete("key").await.unwrap();
        assert!(engine.get("key").await.is_none());
    }

    #[tokio::test]
    async fn test_persistence() {
        let tmp = NamedTempFile::new().unwrap();
        let path = tmp.path().to_path_buf();

        {
            let engine = StorageEngine::new(&path).await.unwrap();
            engine.set("persist".into(), "yes".into()).await.unwrap();
        }

        let engine = StorageEngine::new(&path).await.unwrap();
        assert_eq!(engine.get("persist").await.unwrap(), "yes");
    }
}
