use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn, error};
use serde::{Serialize, Deserialize};
use serde_json::Value;

use crate::error::FerrumError;
use crate::io::{AsyncFileSystem, AsyncFile, DiskFileSystem};
use crate::metrics::Metrics;

use std::time::{SystemTime, Duration, Instant};
use tokio::sync::Mutex;

/// Controls how often writes are flushed to disk for durability.
#[derive(Debug, Clone)]
pub enum FsyncPolicy {
    /// Sync every write (strongest durability).
    Always,
    /// Sync at most once per interval.
    Periodic(Duration),
    /// Never sync automatically (fastest, least durable).
    Never,
}

/// Wrapper for serde_json::Value that serializes as a JSON string in bincode.
/// Bincode doesn't support `deserialize_any` which `serde_json::Value` requires,
/// so we serialize the Value to a JSON string first, then let bincode handle the string.
mod json_value_bincode {
    use serde::{Serializer, Deserializer, Deserialize};
    use serde_json::Value;

    pub fn serialize<S: Serializer>(value: &Value, serializer: S) -> Result<S::Ok, S::Error> {
        let json_str = serde_json::to_string(value).map_err(serde::ser::Error::custom)?;
        serializer.serialize_str(&json_str)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Value, D::Error> {
        let json_str = String::deserialize(deserializer)?;
        serde_json::from_str(&json_str).map_err(serde::de::Error::custom)
    }
}

/// Types of operations stored in the log.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum LogOp {
    Set {
        key: String,
        #[serde(with = "json_value_bincode")]
        value: Value,
        expiry: Option<SystemTime>
    },
    Delete { key: String },
    /// A group of operations that must be applied together.
    Transaction { ops: Vec<LogOp> },
}

/// Index entry storing only the file offset and size — values are read from disk on demand.
#[derive(Clone, Debug)]
struct IndexEntry {
    /// Byte offset in the log file where this record's data begins (after the length prefix).
    offset: u64,
    /// Size of the serialized LogOp bincode data.
    size: u64,
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

/// Core key-value storage engine with in-memory offset index and append-only log.
/// Values are NOT stored in memory — only file offsets. Reads go to disk.
pub struct StorageEngine {
    /// In-memory index: key -> IndexEntry (offset + size, no values)
    index: RwLock<HashMap<String, IndexEntry>>,
    /// Handle to the log file for appending
    log_file: RwLock<Box<dyn AsyncFile>>,
    /// Current write offset in the log file (tracks append position)
    write_offset: Mutex<u64>,
    /// Secondary indexes: IndexName -> {ValueAsString -> [Keys]}
    secondary_indexes: RwLock<HashMap<String, HashMap<String, Vec<String>>>>,
    /// Filesystem abstraction
    fs: Box<dyn AsyncFileSystem>,
    /// Path to the database file
    path: PathBuf,
    /// Durability policy for log writes
    fsync_policy: FsyncPolicy,
    /// Last time data was synced (for periodic policy)
    last_sync: Mutex<Instant>,
    /// Operation metrics for observability
    metrics: Arc<Metrics>,
}

impl StorageEngine {
    /// Create a new storage engine, recovering the index from the log file.
    pub async fn new(path: impl AsRef<std::path::Path>) -> Result<Self, FerrumError> {
        Self::with_fs(path, Box::new(DiskFileSystem)).await
    }

    pub async fn with_fs(path: impl AsRef<std::path::Path>, fs: Box<dyn AsyncFileSystem>) -> Result<Self, FerrumError> {
        Self::with_fs_and_policy(path, fs, FsyncPolicy::Always).await
    }

    pub async fn with_fs_and_policy(
        path: impl AsRef<std::path::Path>,
        fs: Box<dyn AsyncFileSystem>,
        fsync_policy: FsyncPolicy,
    ) -> Result<Self, FerrumError> {
        let path = path.as_ref().to_path_buf();
        let mut index = HashMap::new();

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            fs.create_dir_all(parent).await.map_err(FerrumError::Io)?;
        }

        // Open or create the log file
        let file = fs.open_append(&path).await.map_err(FerrumError::Io)?;

        let mut write_offset: u64 = 0;

        // Recovery: Read the entire file to rebuild the offset index
        if fs.exists(&path).await {
            let file_len = file.metadata_len().await.map_err(FerrumError::Io)?;
            if file_len > 0 {
                info!("Recovering FerrumDB from log: {} ({} bytes)", path.display(), file_len);

                let mut reader = fs.open_read(&path).await.map_err(FerrumError::Io)?;
                let mut buffer = Vec::new();
                reader.read_to_end(&mut buffer).await.map_err(FerrumError::Io)?;

                let mut cursor: u64 = 0;
                while (cursor as usize) < buffer.len() {
                    // Read size prefix (u64)
                    if (cursor as usize) + 8 > buffer.len() { break; }
                    let size_bytes: [u8; 8] = buffer[cursor as usize..(cursor as usize) + 8].try_into().unwrap();
                    let size = u64::from_le_bytes(size_bytes);
                    let data_offset = cursor + 8;

                    if (data_offset as usize) + (size as usize) > buffer.len() {
                        warn!("Incomplete record at end of log, truncating during next write.");
                        break;
                    }

                    let entry_data = &buffer[data_offset as usize..(data_offset as usize) + (size as usize)];
                    match bincode::deserialize::<LogOp>(entry_data) {
                        Ok(op) => {
                            Self::apply_op_to_index(&mut index, &op, data_offset, size);
                        }
                        Err(e) => {
                            error!("Failed to deserialize log entry at offset {}: {}", cursor, e);
                            return Err(FerrumError::Corruption(format!("At offset {}: {}", cursor, e)));
                        }
                    }
                    cursor = data_offset + size;
                }
                write_offset = cursor;
            }
        }

        info!("Storage engine initialized with {} entries (offset-based index)", index.len());

        Ok(Self {
            index: RwLock::new(index),
            log_file: RwLock::new(file),
            write_offset: Mutex::new(write_offset),
            secondary_indexes: RwLock::new(HashMap::new()),
            fs,
            path,
            fsync_policy,
            last_sync: Mutex::new(Instant::now()),
            metrics: Arc::new(Metrics::new()),
        })
    }

    /// Apply a LogOp to the in-memory offset index during recovery.
    fn apply_op_to_index(index: &mut HashMap<String, IndexEntry>, op: &LogOp, data_offset: u64, size: u64) {
        match op {
            LogOp::Set { key, expiry, .. } => {
                index.insert(key.clone(), IndexEntry { offset: data_offset, size, expiry: *expiry });
            }
            LogOp::Delete { key } => {
                index.remove(key);
            }
            LogOp::Transaction { ops } => {
                // For transactions, each sub-op is inside the same serialized blob.
                // We store the transaction's offset for each SET key — on read, we
                // deserialize the full transaction and extract the value.
                for sub_op in ops {
                    match sub_op {
                        LogOp::Set { key, expiry, .. } => {
                            index.insert(key.clone(), IndexEntry { offset: data_offset, size, expiry: *expiry });
                        }
                        LogOp::Delete { key } => {
                            index.remove(key);
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    /// Read a value from disk given an index entry.
    async fn read_value_at(&self, entry: &IndexEntry, key: &str) -> Result<Option<Value>, FerrumError> {
        let data = self.fs.read_at(&self.path, entry.offset, entry.size as usize)
            .await.map_err(FerrumError::Io)?;

        let op: LogOp = bincode::deserialize(&data)
            .map_err(|e| FerrumError::Corruption(format!("Failed to read value for key '{}': {}", key, e)))?;

        match op {
            LogOp::Set { value, .. } => Ok(Some(value)),
            LogOp::Transaction { ops } => {
                // Find the last SET for this key within the transaction
                for sub_op in ops.into_iter().rev() {
                    match sub_op {
                        LogOp::Set { key: k, value, .. } if k == key => return Ok(Some(value)),
                        LogOp::Delete { key: k } if k == key => return Ok(None),
                        _ => {}
                    }
                }
                Ok(None)
            }
            _ => Ok(None),
        }
    }

    /// Retrieve a value from the database by its key.
    /// Returns `None` if the key doesn't exist or has expired.
    pub async fn get(&self, key: &str) -> Option<Value> {
        self.metrics.record_get();
        let entry = {
            let index = self.index.read().await;
            index.get(key).cloned()
        };

        let entry = entry?;

        // Check expiry
        if let Some(expiry) = entry.expiry {
            if SystemTime::now() > expiry {
                return None;
            }
        }

        // Read value from disk
        match self.read_value_at(&entry, key).await {
            Ok(val) => val,
            Err(e) => {
                error!("Failed to read value for key '{}': {}", key, e);
                None
            }
        }
    }

    /// Store a JSON value with no expiration.
    pub async fn set(&self, key: String, value: Value) -> Result<Option<Value>, FerrumError> {
        self.set_ex(key, value, None).await
    }

    /// Store a JSON value with an optional Time-To-Live (TTL).
    pub async fn set_ex(&self, key: String, value: Value, ttl: Option<Duration>) -> Result<Option<Value>, FerrumError> {
        self.metrics.record_set();
        let expiry = ttl.map(|t| SystemTime::now() + t);
        let op = LogOp::Set {
            key: key.clone(),
            value: value.clone(),
            expiry
        };

        // Serialize and write, capturing the offset
        let encoded = bincode::serialize(&op).map_err(|e| FerrumError::Corruption(e.to_string()))?;
        let size = encoded.len() as u64;
        let data_offset = self.append_raw_to_log(&encoded).await?;

        let entry = IndexEntry { offset: data_offset, size, expiry };

        // Read old value for secondary index update (must happen before index update)
        let old_val = {
            let index = self.index.read().await;
            if let Some(old_entry) = index.get(&key) {
                self.read_value_at(old_entry, &key).await.ok().flatten()
            } else {
                None
            }
        };

        {
            let mut index = self.index.write().await;
            index.insert(key.clone(), entry);
        }

        // Update secondary indexes
        self.update_secondary_indexes(&key, old_val.as_ref(), Some(&value)).await;

        Ok(old_val)
    }

    /// Remove a key-value pair from the database.
    /// Returns the deleted value if it existed.
    pub async fn delete(&self, key: &str) -> Result<Option<Value>, FerrumError> {
        self.metrics.record_delete();

        // Read old value before removing from index
        let old_val = {
            let index = self.index.read().await;
            if let Some(entry) = index.get(key) {
                self.read_value_at(entry, key).await.ok().flatten()
            } else {
                None
            }
        };

        if old_val.is_some() {
            let op = LogOp::Delete { key: key.to_string() };
            let encoded = bincode::serialize(&op).map_err(|e| FerrumError::Corruption(e.to_string()))?;
            self.append_raw_to_log(&encoded).await?;

            {
                let mut index = self.index.write().await;
                index.remove(key);
            }

            self.update_secondary_indexes(key, old_val.as_ref(), None).await;
        }

        Ok(old_val)
    }

    /// Create a secondary index on a specific JSON field.
    pub async fn create_index(&self, field: &str) -> Result<(), FerrumError> {
        let mut sec_indexes = self.secondary_indexes.write().await;
        let mut new_index: HashMap<String, Vec<String>> = HashMap::new();

        // Must read values from disk to build the secondary index
        let entries: Vec<(String, IndexEntry)> = {
            let index = self.index.read().await;
            index.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
        };

        for (key, entry) in &entries {
            if let Ok(Some(value)) = self.read_value_at(entry, key).await {
                if let Some(val) = value.get(field) {
                    let val_str = val.to_string();
                    new_index.entry(val_str).or_default().push(key.clone());
                }
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
        let encoded = bincode::serialize(&tx_op).map_err(|e| FerrumError::Corruption(e.to_string()))?;
        let size = encoded.len() as u64;
        let data_offset = self.append_raw_to_log(&encoded).await?;

        // Apply all ops to memory index under a single write lock for consistency
        let mut index = self.index.write().await;
        let mut sec_indexes = self.secondary_indexes.write().await;

        for op in ops {
            match op {
                LogOp::Set { key, value, expiry } => {
                    // Read old value for secondary index update
                    let old_val = if let Some(old_entry) = index.get(&key) {
                        self.read_value_at(old_entry, &key).await.ok().flatten()
                    } else {
                        None
                    };

                    index.insert(key.clone(), IndexEntry { offset: data_offset, size, expiry });
                    Self::update_secondary_indexes_internal(&mut sec_indexes, &key, old_val.as_ref(), Some(&value));
                }
                LogOp::Delete { key } => {
                    if let Some(old_entry) = index.remove(&key) {
                        let old_val = self.read_value_at(&old_entry, &key).await.ok().flatten();
                        Self::update_secondary_indexes_internal(&mut sec_indexes, &key, old_val.as_ref(), None);
                    }
                }
                LogOp::Transaction { .. } => {
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

    /// Returns a reference to the metrics tracker.
    pub fn metrics(&self) -> &Metrics {
        &self.metrics
    }

    /// Appends raw serialized data to the log with a length prefix.
    /// Returns the byte offset where the data (not the length prefix) was written.
    async fn append_raw_to_log(&self, encoded: &[u8]) -> Result<u64, FerrumError> {
        let size = encoded.len() as u64;
        let size_bytes = size.to_le_bytes();

        // Combine size prefix and data into a single write for atomicity
        let mut combined = Vec::with_capacity(8 + encoded.len());
        combined.extend_from_slice(&size_bytes);
        combined.extend_from_slice(encoded);

        let mut file = self.log_file.write().await;
        let mut offset = self.write_offset.lock().await;

        let data_offset = *offset + 8; // data starts after the 8-byte length prefix

        file.write_all(&combined).await.map_err(FerrumError::Io)?;
        *offset += combined.len() as u64;

        // Fsync per policy
        match self.fsync_policy {
            FsyncPolicy::Always => {
                file.sync_data().await.map_err(FerrumError::Io)?;
            }
            FsyncPolicy::Never => {}
            FsyncPolicy::Periodic(interval) => {
                let mut last_sync = self.last_sync.lock().await;
                if last_sync.elapsed() >= interval {
                    file.sync_data().await.map_err(FerrumError::Io)?;
                    *last_sync = Instant::now();
                }
            }
        }

        Ok(data_offset)
    }

    /// Compacts the log file by writing only live data to a new file.
    pub async fn compact(&self, current_path: impl AsRef<Path>) -> Result<(), FerrumError> {
        let current_path = current_path.as_ref();
        let temp_path = current_path.with_extension("db.tmp");

        // 1. Create temp file
        let mut temp_file = self.fs.open_write(&temp_path).await.map_err(FerrumError::Io)?;

        // 2. Collect live entries and read their values from disk
        let live_entries: Vec<(String, IndexEntry)> = {
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

        // 3. Write live data to temp file and build new index
        let mut new_index = HashMap::new();
        let mut new_offset: u64 = 0;

        for (key, entry) in &live_entries {
            if let Ok(Some(value)) = self.read_value_at(entry, key).await {
                let op = LogOp::Set {
                    key: key.clone(),
                    value,
                    expiry: entry.expiry
                };
                let encoded = bincode::serialize(&op).map_err(|e| FerrumError::Corruption(e.to_string()))?;
                let size = encoded.len() as u64;
                let size_bytes = size.to_le_bytes();

                // Write as single combined block
                let mut combined = Vec::with_capacity(8 + encoded.len());
                combined.extend_from_slice(&size_bytes);
                combined.extend_from_slice(&encoded);

                let data_offset = new_offset + 8;
                temp_file.write_all(&combined).await.map_err(FerrumError::Io)?;

                new_index.insert(key.clone(), IndexEntry {
                    offset: data_offset,
                    size,
                    expiry: entry.expiry,
                });

                new_offset += combined.len() as u64;
            }
        }
        temp_file.sync_all().await.map_err(FerrumError::Io)?;

        // 4. Atomic swap under write lock
        {
            let mut log_file = self.log_file.write().await;
            let mut write_offset = self.write_offset.lock().await;

            self.fs.rename(&temp_path, current_path).await.map_err(FerrumError::Io)?;
            *log_file = self.fs.open_append(current_path).await.map_err(FerrumError::Io)?;
            *write_offset = new_offset;
        }

        // 5. Update the in-memory index with new offsets
        {
            let mut index = self.index.write().await;
            *index = new_index;
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

    #[tokio::test]
    async fn test_encryption() {
        use crate::io::EncryptedFileSystem;
        let tmp = NamedTempFile::new().unwrap();
        let path = tmp.path().to_path_buf();
        let key = [7u8; 32];

        // 1. Write encrypted data
        {
            let fs = Box::new(EncryptedFileSystem::new(Box::new(DiskFileSystem), key));
            let engine = StorageEngine::with_fs(&path, fs).await.unwrap();
            engine.set("secret".into(), serde_json::json!("sensitive data")).await.unwrap();
        }

        // 2. Verify raw file is encrypted (should not contain plaintext "sensitive data")
        let raw_bytes = std::fs::read(&path).unwrap();
        let raw_str = String::from_utf8_lossy(&raw_bytes);
        assert!(!raw_str.contains("sensitive data"));

        // 3. Recover with correct key
        {
            let fs = Box::new(EncryptedFileSystem::new(Box::new(DiskFileSystem), key));
            let engine = StorageEngine::with_fs(&path, fs).await.unwrap();
            assert_eq!(engine.get("secret").await.unwrap(), serde_json::json!("sensitive data"));
        }

        // 4. Recovery fails/panics with wrong key
        {
            let wrong_key = [9u8; 32];
            let fs = Box::new(EncryptedFileSystem::new(Box::new(DiskFileSystem), wrong_key));
            let result = StorageEngine::with_fs(&path, fs).await;
            // It should fail during recovery decryption
            assert!(result.is_err());
        }
    }
}
