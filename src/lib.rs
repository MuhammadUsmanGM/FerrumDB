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
    engine: StorageEngine,
}

impl FerrumDB {
    /// Open the database with default configuration (zero-setup).
    pub async fn open_default() -> Result<Self, FerrumError> {
        let config = Config::default();
        Self::open(config).await
    }

    /// Open the database with a specific configuration.
    pub async fn open(config: Config) -> Result<Self, FerrumError> {
        let engine = StorageEngine::new(config.path).await?;
        Ok(Self { engine })
    }

    pub fn engine(&self) -> &StorageEngine {
        &self.engine
    }
}
