use std::collections::HashMap;
use std::path::Path;
use tokio::fs::{self, File, OpenOptions};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::RwLock;
use tracing::{info, warn, error};
use serde::{Serialize, Deserialize};
use serde_json::Value;

use crate::error::FerrumError;

use std::time::{SystemTime, Duration};

/// Types of operations stored in the log.
#[derive(Serialize, Deserialize, Debug)]
enum LogOp {
    Set { 
        key: String, 
        value: Value, 
        expiry: Option<SystemTime> 
    },
    Delete { key: String },
}

/// Value stored in the in-memory index.
#[derive(Clone, Debug)]
struct ValueEntry {
    value: Value,
    expiry: Option<SystemTime>,
}

/// Core key-value storage engine with in-memory index and append-only log.
pub struct StorageEngine {
    /// In-memory index: key -> ValueEntry
    index: RwLock<HashMap<String, ValueEntry>>,
    /// Handle to the log file for appending
    log_file: RwLock<File>,
}

impl StorageEngine {
    /// Create a new storage engine, recovering the index from the log file.
    pub async fn new(path: impl AsRef<Path>) -> Result<Self, FerrumError> {
        let path = path.as_ref().to_path_buf();
        let mut index = HashMap::new();

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await.map_err(FerrumError::Io)?;
        }

        // Open or create the log file
        let file = OpenOptions::new()
            .read(true)
            .append(true)
            .create(true)
            .open(&path)
            .await
            .map_err(FerrumError::Io)?;

        // Recovery: Read the entire file to rebuild the index
        let file_len = file.metadata().await.map_err(FerrumError::Io)?.len();
        if file_len > 0 {
            info!("Recovering FerrumDB from log: {} ({} bytes)", path.display(), file_len);
            
            // Re-open for reading from the start for recovery
            let mut reader = File::open(&path).await.map_err(FerrumError::Io)?;
            let mut buffer = Vec::new();
            reader.read_to_end(&mut buffer).await.map_err(FerrumError::Io)?;

            let mut cursor = 0;
            while cursor < buffer.len() {
                // Peek size (u64)
                if cursor + 8 > buffer.len() { break; }
                let size_bytes: [u8; 8] = buffer[cursor..cursor + 8].try_into().unwrap();
                let size = u64::from_le_bytes(size_bytes) as usize;
                cursor += 8;

                if cursor + size > buffer.len() {
                    warn!("Incomplete record at end of log, truncating during next write.");
                    break;
                }

                let entry_data = &buffer[cursor..cursor + size];
                match bincode::deserialize::<LogOp>(entry_data) {
                    Ok(op) => {
                        match op {
                            LogOp::Set { key, value, expiry } => { 
                                index.insert(key, ValueEntry { value, expiry }); 
                            }
                            LogOp::Delete { key } => { index.remove(&key); }
                        }
                    }
                    Err(e) => {
                        error!("Failed to deserialize log entry at offset {}: {}", cursor, e);
                        return Err(FerrumError::Corruption(format!("At offset {}: {}", cursor, e)));
                    }
                }
                cursor += size;
            }
        }

        info!("Storage engine initialized with {} entries", index.len());

        Ok(Self {
            index: RwLock::new(index),
            log_file: RwLock::new(file),
        })
    }

    pub async fn get(&self, key: &str) -> Option<Value> {
        let index = self.index.read().await;
        if let Some(entry) = index.get(key) {
            if let Some(expiry) = entry.expiry {
                if SystemTime::now() > expiry {
                    return None;
                }
            }
            return Some(entry.value.clone());
        }
        None
    }

    pub async fn set(&self, key: String, value: Value) -> Result<Option<Value>, FerrumError> {
        self.set_ex(key, value, None).await
    }

    /// Set with optional TTL.
    pub async fn set_ex(&self, key: String, value: Value, ttl: Option<Duration>) -> Result<Option<Value>, FerrumError> {
        let expiry = ttl.map(|t| SystemTime::now() + t);
        let op = LogOp::Set { 
            key: key.clone(), 
            value: value.clone(),
            expiry 
        };
        self.append_to_log(op).await?;

        let entry = ValueEntry { value, expiry };
        let mut index = self.index.write().await;
        Ok(index.insert(key, entry).map(|e| e.value))
    }

    pub async fn delete(&self, key: &str) -> Result<Option<Value>, FerrumError> {
        let op = LogOp::Delete { key: key.to_string() };
        
        let mut index = self.index.write().await;
        let old = index.remove(key).map(|e| e.value);
        
        if old.is_some() {
            self.append_to_log(op).await?;
        }
        
        Ok(old)
    }

    pub async fn keys(&self) -> Vec<String> {
        let index = self.index.read().await;
        index.keys().cloned().collect()
    }

    pub async fn len(&self) -> usize {
        let index = self.index.read().await;
        index.len()
    }

    /// Appends a serialized operation to the end of the log file.
    /// Format: [Length (u64)][Serialized Data]
    async fn append_to_log(&self, op: LogOp) -> Result<(), FerrumError> {
        let encoded = bincode::serialize(&op).map_err(FerrumError::Bincode)?;
        let size = encoded.len() as u64;
        let size_bytes = size.to_le_bytes();

        let mut file = self.log_file.write().await;
        
        // Write size prefix then data
        file.write_all(&size_bytes).await.map_err(FerrumError::Io)?;
        file.write_all(&encoded).await.map_err(FerrumError::Io)?;
        
        // Ensure data hit the disk
        file.sync_data().await.map_err(FerrumError::Io)?;
        
        Ok(())
    }

    /// Compasts the log file by writing only live data to a new file.
    pub async fn compact(&self, current_path: impl AsRef<Path>) -> Result<(), FerrumError> {
        let current_path = current_path.as_ref();
        let temp_path = current_path.with_extension("db.tmp");

        // 1. Create temp file
        let mut temp_file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&temp_path)
            .await
            .map_err(FerrumError::Io)?;

        // 2. Collect live data under read lock
        let live_data: Vec<(String, ValueEntry)> = {
            let index = self.index.read().await;
            index.iter()
                .filter(|(_, entry)| {
                    if let Some(expiry) = entry.expiry {
                        SystemTime::now() < expiry
                    } else {
                        true
                    }
                })
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect()
        };

        // 3. Write live data to temp file
        for (key, entry) in live_data {
            let op = LogOp::Set { 
                key, 
                value: entry.value, 
                expiry: entry.expiry 
            };
            let encoded = bincode::serialize(&op).map_err(FerrumError::Bincode)?;
            let size = encoded.len() as u64;
            temp_file.write_all(&size.to_le_bytes()).await.map_err(FerrumError::Io)?;
            temp_file.write_all(&encoded).await.map_err(FerrumError::Io)?;
        }
        temp_file.sync_all().await.map_err(FerrumError::Io)?;

        // 4. Atomic swap under write lock
        {
            let mut log_file = self.log_file.write().await;
            // Rename is atomic on most OSs
            fs::rename(&temp_path, &current_path).await.map_err(FerrumError::Io)?;
            
            // Re-open log file handle
            *log_file = OpenOptions::new()
                .read(true)
                .append(true)
                .open(&current_path)
                .await
                .map_err(FerrumError::Io)?;
        }

        info!("Compaction completed. Live entries: {}", self.len().await);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_set_get_delete() {
        let tmp = NamedTempFile::new().unwrap();
        let engine = StorageEngine::new(tmp.path()).await.unwrap();

        assert!(engine.get("key").await.is_none());

        engine.set("key".into(), "value".into()).await.unwrap();
        assert_eq!(engine.get("key").await.unwrap(), serde_json::json!("value"));

        engine.delete("key").await.unwrap();
        assert!(engine.get("key").await.is_none());
    }

    #[tokio::test]
    async fn test_recovery() {
        let tmp = NamedTempFile::new().unwrap();
        let path = tmp.path().to_path_buf();

        {
            let engine = StorageEngine::new(&path).await.unwrap();
            engine.set("k1".into(), "v1".into()).await.unwrap();
            engine.set("k2".into(), "v2".into()).await.unwrap();
            engine.delete("k1").await.unwrap();
        }

        // Recover
        let engine = StorageEngine::new(&path).await.unwrap();
        assert_eq!(engine.get("k2").await.unwrap(), serde_json::json!("v2"));
        assert!(engine.get("k1").await.is_none());
        assert_eq!(engine.len().await, 1);
    }

    #[tokio::test]
    async fn test_concurrent_access() {
        let tmp = NamedTempFile::new().unwrap();
        let engine = Arc::new(StorageEngine::new(tmp.path()).await.unwrap());
        
        let mut handles = Vec::new();
        for i in 0..100 {
            let e = Arc::clone(&engine);
            handles.push(tokio::spawn(async move {
                e.set(format!("k{i}"), format!("v{i}").into()).await.unwrap();
            }));
        }

        for h in handles {
            h.await.unwrap();
        }

        assert_eq!(engine.len().await, 100);
        assert_eq!(engine.get("k50").await.unwrap(), serde_json::json!("v50"));
    }
}
