//! # FerrumDB
//! 
//! A premium, high-performance embedded key-value database for Rust applications.
//! 
//! FerrumDB focuses on being **zero-setup**, **performant**, and **developer-friendly**.
//! It uses an append-only log (AOF) for $O(1)$ writes and maintains an in-memory 
//! index for $O(1)$ reads.
//! 
//! ## Quick Start
//! ```rust
//! use ferrumdb::FerrumDB;
//! use serde_json::json;
//! 
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let db = FerrumDB::open_default().await?;
//! db.set("key".into(), json!({"value": 42})).await?;
//! # Ok(())
//! # }
//! ```

pub mod storage;
pub mod error;
pub mod metrics;
pub mod cli;
pub mod io;

pub use storage::{StorageEngine, Transaction};
pub use error::FerrumError;
pub use metrics::Metrics;
pub use io::{AsyncFileSystem, DiskFileSystem, EncryptedFileSystem};

use std::path::PathBuf;

/// High-level configuration for FerrumDB.
pub struct Config {
    pub path: PathBuf,
    pub encryption_key: Option<[u8; 32]>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            path: PathBuf::from("ferrum.db"),
            encryption_key: None,
        }
    }
}

impl Config {
    /// Create a new config for the default path.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set an encryption key (AES-256 requires 32 bytes).
    pub fn with_encryption(mut self, key: [u8; 32]) -> Self {
        self.encryption_key = Some(key);
        self
    }
}

/// The premium FerrumDB instance.
pub struct FerrumDB {
    engine: std::sync::Arc<StorageEngine>,
}

impl FerrumDB {
    /// Open the database with default configuration (zero-setup).
    /// Defaults to `./ferrum.db`.
    pub async fn open_default() -> Result<Self, FerrumError> {
        let config = Config::default();
        Self::open(config).await
    }

    /// Open the database with a specific configuration.
    pub async fn open(config: Config) -> Result<Self, FerrumError> {
        use crate::io::{AsyncFileSystem, DiskFileSystem, EncryptedFileSystem};
        
        let mut fs: Box<dyn AsyncFileSystem> = Box::new(DiskFileSystem);
        if let Some(key) = config.encryption_key {
            fs = Box::new(EncryptedFileSystem::new(fs, key));
        }

        let engine = StorageEngine::with_fs(config.path, fs).await?;
        Ok(Self { 
            engine: std::sync::Arc::new(engine) 
        })
    }

    /// Get the underlying storage engine.
    pub fn engine(&self) -> std::sync::Arc<StorageEngine> {
        self.engine.clone()
    }

    /// High-level access to SET a JSON value.
    pub async fn set(&self, key: String, value: serde_json::Value) -> Result<Option<serde_json::Value>, FerrumError> {
        self.engine.set(key, value).await
    }

    /// High-level access to GET a JSON value.
    pub async fn get(&self, key: &str) -> Option<serde_json::Value> {
        self.engine.get(key).await
    }

    /// Create a secondary index on a specific JSON field.
    pub async fn create_index(&self, field: &str) -> Result<(), FerrumError> {
        self.engine.create_index(field).await
    }

    /// Search the database using a secondary index.
    pub async fn find(&self, field: &str, value: &serde_json::Value) -> Vec<String> {
        self.engine.get_by_index(field, value).await
    }

    /// Commit a batch of operations atomically using a Transaction.
    pub async fn commit(&self, tx: Transaction) -> Result<(), FerrumError> {
        self.engine.commit_transaction(tx.build()).await
    }
}
