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
    use std::sync::Arc;
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

    #[tokio::test]
    async fn test_overwrite_value() {
        let tmp = NamedTempFile::new().unwrap();
        let engine = StorageEngine::new(tmp.path()).await.unwrap();

        engine.set("k".into(), "v1".into()).await.unwrap();
        let old = engine.set("k".into(), "v2".into()).await.unwrap();
        assert_eq!(old, Some("v1".to_string()));
        assert_eq!(engine.get("k").await.unwrap(), "v2");
    }

    #[tokio::test]
    async fn test_delete_nonexistent() {
        let tmp = NamedTempFile::new().unwrap();
        let engine = StorageEngine::new(tmp.path()).await.unwrap();

        let result = engine.delete("nope").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_keys_and_len() {
        let tmp = NamedTempFile::new().unwrap();
        let engine = StorageEngine::new(tmp.path()).await.unwrap();

        assert_eq!(engine.len().await, 0);
        engine.set("a".into(), "1".into()).await.unwrap();
        engine.set("b".into(), "2".into()).await.unwrap();
        assert_eq!(engine.len().await, 2);

        let mut keys = engine.keys().await;
        keys.sort();
        assert_eq!(keys, vec!["a", "b"]);
    }

    /// Stress test: 1000 concurrent writes followed by reads.
    #[tokio::test]
    #[ignore] // Run with: cargo test --release -- --ignored
    async fn stress_test_concurrent_writes() {
        let tmp = NamedTempFile::new().unwrap();
        let engine = Arc::new(StorageEngine::new(tmp.path()).await.unwrap());
        let num_tasks = 1000;

        // Concurrent writes
        let mut handles = Vec::new();
        for i in 0..num_tasks {
            let eng = Arc::clone(&engine);
            handles.push(tokio::spawn(async move {
                eng.set(format!("key_{i}"), format!("val_{i}")).await.unwrap();
            }));
        }
        for h in handles {
            h.await.unwrap();
        }

        assert_eq!(engine.len().await, num_tasks);

        // Verify all values
        for i in 0..num_tasks {
            let val = engine.get(&format!("key_{i}")).await.unwrap();
            assert_eq!(val, format!("val_{i}"));
        }
    }

    /// Stress test: concurrent reads and writes interleaved.
    #[tokio::test]
    #[ignore]
    async fn stress_test_mixed_read_write() {
        let tmp = NamedTempFile::new().unwrap();
        let engine = Arc::new(StorageEngine::new(tmp.path()).await.unwrap());

        // Seed some data
        for i in 0..100 {
            engine.set(format!("k{i}"), format!("v{i}")).await.unwrap();
        }

        let mut handles = Vec::new();

        // 500 concurrent reads
        for i in 0..500 {
            let eng = Arc::clone(&engine);
            handles.push(tokio::spawn(async move {
                let _ = eng.get(&format!("k{}", i % 100)).await;
            }));
        }

        // 500 concurrent writes
        for i in 0..500 {
            let eng = Arc::clone(&engine);
            handles.push(tokio::spawn(async move {
                eng.set(format!("mix_{i}"), format!("val_{i}")).await.unwrap();
            }));
        }

        for h in handles {
            h.await.unwrap();
        }

        // Original keys intact
        for i in 0..100 {
            assert_eq!(engine.get(&format!("k{i}")).await.unwrap(), format!("v{i}"));
        }
        // New keys written
        assert_eq!(engine.len().await, 600);
    }

    /// Stress test: persistence survives after bulk operations.
    #[tokio::test]
    #[ignore]
    async fn stress_test_persistence_after_bulk() {
        let tmp = NamedTempFile::new().unwrap();
        let path = tmp.path().to_path_buf();

        {
            let engine = Arc::new(StorageEngine::new(&path).await.unwrap());
            let mut handles = Vec::new();
            for i in 0..500 {
                let eng = Arc::clone(&engine);
                handles.push(tokio::spawn(async move {
                    eng.set(format!("bulk_{i}"), format!("data_{i}")).await.unwrap();
                }));
            }
            for h in handles {
                h.await.unwrap();
            }
        }

        // Reload from disk
        let engine = StorageEngine::new(&path).await.unwrap();
        assert_eq!(engine.len().await, 500);
        assert_eq!(engine.get("bulk_0").await.unwrap(), "data_0");
        assert_eq!(engine.get("bulk_499").await.unwrap(), "data_499");
    }
}
