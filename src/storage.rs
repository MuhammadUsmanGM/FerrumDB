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
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum LogOp {
    Set { 
        key: String, 
        value: Value, 
        expiry: Option<SystemTime> 
    },
    Delete { key: String },
    /// A group of operations that must be applied together.
    Transaction { ops: Vec<LogOp> },
}

/// Value stored in the in-memory index.
#[derive(Clone, Debug)]
struct ValueEntry {
    value: Value,
    expiry: Option<SystemTime>,
}

/// A builder for atomic transactions.
pub struct Transaction {
    ops: Vec<LogOp>,
}

impl Transaction {
    /// Create a new empty transaction.
    pub fn new() -> Self {
        Self { ops: Vec::new() }
    }

    /// Add a SET operation to the transaction.
    pub fn set(mut self, key: String, value: Value) -> Self {
        self.ops.push(LogOp::Set { key, value, expiry: None });
        self
    }

    /// Add a SET operation with TTL to the transaction.
    pub fn set_ex(mut self, key: String, value: Value, ttl: Duration) -> Self {
        let expiry = Some(SystemTime::now() + ttl);
        self.ops.push(LogOp::Set { key, value, expiry });
        self
    }

    /// Add a DELETE operation to the transaction.
    pub fn delete(mut self, key: String) -> Self {
        self.ops.push(LogOp::Delete { key });
        self
    }

    /// Consumes the builder and returns the operations.
    pub fn build(self) -> Vec<LogOp> {
        self.ops
    }
}

/// Core key-value storage engine with in-memory index and append-only log.
pub struct StorageEngine {
    /// In-memory primary index: key -> ValueEntry
    index: RwLock<HashMap<String, ValueEntry>>,
    /// Handle to the log file for appending
    log_file: RwLock<File>,
    /// Secondary indexes: IndexName -> {ValueAsString -> [Keys]}
    secondary_indexes: RwLock<HashMap<String, HashMap<String, Vec<String>>>>,
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
                match serde_json::from_slice::<LogOp>(entry_data) {
                    Ok(op) => {
                        Self::apply_op_to_index(&mut index, op);
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
            secondary_indexes: RwLock::new(HashMap::new()),
        })
    }

    /// Helper to apply a LogOp to the in-memory index.
    fn apply_op_to_index(index: &mut HashMap<String, ValueEntry>, op: LogOp) {
        match op {
            LogOp::Set { key, value, expiry } => {
                index.insert(key, ValueEntry { value, expiry });
            }
            LogOp::Delete { key } => {
                index.remove(&key);
            }
            LogOp::Transaction { ops } => {
                for sub_op in ops {
                    Self::apply_op_to_index(index, sub_op);
                }
            }
        }
    }

    /// Retrieve a value from the database by its key.
    /// Returns `None` if the key doesn't exist or has expired.
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

    /// Store a JSON value with no expiration.
    pub async fn set(&self, key: String, value: Value) -> Result<Option<Value>, FerrumError> {
        self.set_ex(key, value, None).await
    }

    /// Store a JSON value with an optional Time-To-Live (TTL).
    pub async fn set_ex(&self, key: String, value: Value, ttl: Option<Duration>) -> Result<Option<Value>, FerrumError> {
        let expiry = ttl.map(|t| SystemTime::now() + t);
        let op = LogOp::Set { 
            key: key.clone(), 
            value: value.clone(),
            expiry 
        };
        self.append_to_log(op).await?;

        let entry = ValueEntry { value: value.clone(), expiry };
        
        let old_val = {
            let mut index = self.index.write().await;
            index.insert(key.clone(), entry).map(|e| e.value)
        };

        // Update secondary indexes
        self.update_secondary_indexes(&key, old_val.as_ref(), Some(&value)).await;

        Ok(old_val)
    }

    /// Remove a key-value pair from the database.
    /// Returns the deleted value if it existed.
    pub async fn delete(&self, key: &str) -> Result<Option<Value>, FerrumError> {
        let op = LogOp::Delete { key: key.to_string() };
        
        let old_val = {
            let mut index = self.index.write().await;
            index.remove(key).map(|e| e.value)
        };
        
        if let Some(val) = &old_val {
            self.append_to_log(op).await?;
            self.update_secondary_indexes(key, Some(val), None).await;
        }
        
        Ok(old_val)
    }

    /// Create a secondary index on a specific JSON field.
    pub async fn create_index(&self, field: &str) -> Result<(), FerrumError> {
        let mut sec_indexes = self.secondary_indexes.write().await;
        let mut new_index: HashMap<String, Vec<String>> = HashMap::new();

        let index = self.index.read().await;
        for (key, entry) in index.iter() {
            if let Some(val) = entry.value.get(field) {
                let val_str = val.to_string();
                new_index.entry(val_str).or_default().push(key.clone());
            }
        }

        sec_indexes.insert(field.to_string(), new_index);
        info!("Created secondary index on field: '{}'", field);
        Ok(())
    }

    /// Get all keys that match a specific value in a secondary index.
    pub async fn get_by_index(&self, field: &str, value: &Value) -> Vec<String> {
        let sec_indexes = self.secondary_indexes.read().await;
        if let Some(index) = sec_indexes.get(field) {
            let val_str = value.to_string();
            if let Some(keys) = index.get(&val_str) {
                return keys.clone();
            }
        }
        Vec::new()
    }

    /// Internal helper to update secondary indexes during SET/DELETE/TX.
    async fn update_secondary_indexes(&self, key: &str, old_val: Option<&Value>, new_val: Option<&Value>) {
        let mut sec_indexes = self.secondary_indexes.write().await;
        Self::update_secondary_indexes_internal(&mut sec_indexes, key, old_val, new_val);
    }

    fn update_secondary_indexes_internal(
        sec_indexes: &mut HashMap<String, HashMap<String, Vec<String>>>,
        key: &str,
        old_val: Option<&Value>,
        new_val: Option<&Value>
    ) {
        for (field, index) in sec_indexes.iter_mut() {
            // Remove old value from index
            if let Some(ov) = old_val {
                if let Some(field_val) = ov.get(field) {
                    let val_str = field_val.to_string();
                    if let Some(keys) = index.get_mut(&val_str) {
                        keys.retain(|k| k != key);
                    }
                }
            }

            // Add new value to index
            if let Some(nv) = new_val {
                if let Some(field_val) = nv.get(field) {
                    let val_str = field_val.to_string();
                    index.entry(val_str).or_default().push(key.to_string());
                }
            }
        }
    }

    /// Commit a batch of operations atomically.
    pub async fn commit_transaction(&self, ops: Vec<LogOp>) -> Result<(), FerrumError> {
        if ops.is_empty() { return Ok(()); }

        let tx_op = LogOp::Transaction { ops: ops.clone() };
        self.append_to_log(tx_op).await?;

        // Apply all ops to memory index under a single write lock for consistency
        let mut index = self.index.write().await;
        let mut sec_indexes = self.secondary_indexes.write().await;

        for op in ops {
            match op {
                LogOp::Set { key, value, expiry } => {
                    let old_val = index.insert(key.clone(), ValueEntry { value: value.clone(), expiry }).map(|e| e.value);
                    Self::update_secondary_indexes_internal(&mut sec_indexes, &key, old_val.as_ref(), Some(&value));
                }
                LogOp::Delete { key } => {
                    if let Some(old_val) = index.remove(&key).map(|e| e.value) {
                        Self::update_secondary_indexes_internal(&mut sec_indexes, &key, Some(&old_val), None);
                    }
                }
                LogOp::Transaction { .. } => {
                    // Nested transactions are not supported via this API for now
                    warn!("Nested transactions found in commit_transaction, skipping sub-ops.");
                }
            }
        }

        Ok(())
    }

    /// Returns a list of all currently indexed keys.
    pub async fn keys(&self) -> Vec<String> {
        let index = self.index.read().await;
        index.keys().cloned().collect()
    }

    /// Returns the total number of entries in the database.
    pub async fn len(&self) -> usize {
        let index = self.index.read().await;
        index.len()
    }

    /// Appends a serialized operation to the end of the log file.
    /// Format: [Length (u64)][Serialized JSON Data]
    async fn append_to_log(&self, op: LogOp) -> Result<(), FerrumError> {
        let encoded = serde_json::to_vec(&op).map_err(|e| FerrumError::Corruption(e.to_string()))?;
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
            let encoded = serde_json::to_vec(&op).map_err(|e| FerrumError::Corruption(e.to_string()))?;
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

    #[tokio::test]
    async fn test_secondary_indexing() {
        let tmp = NamedTempFile::new().unwrap();
        let engine = StorageEngine::new(tmp.path()).await.unwrap();

        // 1. Data Setup
        engine.set("u1".into(), serde_json::json!({"name": "alice", "role": "admin"})).await.unwrap();
        engine.set("u2".into(), serde_json::json!({"name": "bob", "role": "user"})).await.unwrap();
        engine.set("u3".into(), serde_json::json!({"name": "charlie", "role": "admin"})).await.unwrap();

        // 2. Create Index
        engine.create_index("role").await.unwrap();

        // 3. Query Index
        let admins = engine.get_by_index("role", &serde_json::json!("admin")).await;
        assert_eq!(admins.len(), 2);
        assert!(admins.contains(&"u1".to_string()));
        assert!(admins.contains(&"u3".to_string()));

        // 4. Update Data (bob becomes admin)
        engine.set("u2".into(), serde_json::json!({"name": "bob", "role": "admin"})).await.unwrap();
        let admins_updated = engine.get_by_index("role", &serde_json::json!("admin")).await;
        assert_eq!(admins_updated.len(), 3);

        // 5. Delete Data (charlie leaves)
        engine.delete("u3").await.unwrap();
        let admins_final = engine.get_by_index("role", &serde_json::json!("admin")).await;
        assert_eq!(admins_final.len(), 2);
        assert!(!admins_final.contains(&"u3".to_string()));
    }

    #[tokio::test]
    async fn test_transactions() {
        let tmp = NamedTempFile::new().unwrap();
        let engine = StorageEngine::new(tmp.path()).await.unwrap();

        // 1. Create index for secondary index verification during tx
        engine.create_index("tag").await.unwrap();

        // 2. Commit a transaction with multiple operations
        let tx = Transaction::new()
            .set("k1".into(), serde_json::json!({"tag": "blue"}))
            .set("k2".into(), serde_json::json!({"tag": "red"}))
            .delete("k1".into());
        
        engine.commit_transaction(tx.build()).await.unwrap();

        // 3. Verify results
        assert!(engine.get("k1").await.is_none());
        assert!(engine.get("k2").await.is_some());
        
        let red_items = engine.get_by_index("tag", &serde_json::json!("red")).await;
        assert_eq!(red_items.len(), 1);
        assert_eq!(red_items[0], "k2");

        // 4. Recovery Test for Transactions
        let path = tmp.path().to_path_buf();
        drop(engine);
        
        let engine_recovered = StorageEngine::new(&path).await.unwrap();
        assert!(engine_recovered.get("k1").await.is_none());
        assert!(engine_recovered.get("k2").await.is_some());
    }
}
