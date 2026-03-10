//! FerrumDB: A premium, high-performance local key-value database.
//! 
//! Features:
//! - Append-only binary storage (AOF)
//! - JSON Value support (Structured Data)
//! - Time-To-Live (TTL) expiration
//! - Background Compaction
//! - Zero-Setup initialization

pub mod storage;
pub mod error;
pub mod metrics;
pub mod cli;

pub use storage::StorageEngine;
pub use error::FerrumError;
pub use metrics::Metrics;

use std::path::PathBuf;

/// High-level configuration for FerrumDB.
pub struct Config {
    pub path: PathBuf,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            path: PathBuf::from("ferrum.db"),
        }
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
        let engine = StorageEngine::new(config.path).await?;
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
}
