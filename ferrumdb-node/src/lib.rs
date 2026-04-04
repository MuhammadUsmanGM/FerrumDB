//! FerrumDB Node.js Bindings
//!
//! Exposes an API over FerrumDB's storage engine.
//! Uses an internal tokio runtime so Node.js users get synchronous calls.
//!
//! Usage:
//! ```js
//! const { FerrumDB, Transaction } = require('ferrumdb');
//!
//! const db = FerrumDB.open("myapp.db");
//! db.set("user:1", { name: "alice", role: "admin" });
//! const val = db.get("user:1"); // => { name: "alice", role: "admin" }
//!
//! // With TTL (auto-expires after 60 seconds)
//! db.setEx("session:abc", { token: "xyz" }, 60);
//!
//! // With encryption
//! const db2 = FerrumDB.open("secure.db", { encryptionKey: "my_super_secret_key_32_bytes_!!?" });
//! ```

#[macro_use]
extern crate napi_derive;

use napi::{bindgen_prelude::*, JsUnknown};
use std::sync::Arc;
use std::time::Duration;
use ::ferrumdb::{StorageEngine, DiskFileSystem, EncryptedFileSystem, io::AsyncFileSystem};
use serde_json::Value;

/// Convert a serde_json::Value to a napi JsUnknown.
fn value_to_js(env: &Env, val: &Value) -> Result<JsUnknown> {
    env.to_js_value(val)
}

/// Convert a napi JsUnknown to a serde_json::Value.
fn js_to_value(env: &Env, val: JsUnknown) -> Result<Value> {
    env.from_js_value(val)
}

/// The main FerrumDB class for Node.js.
#[napi]
pub struct FerrumDB {
    engine: Arc<StorageEngine>,
    rt: Arc<tokio::runtime::Runtime>,
}

#[napi]
impl FerrumDB {
    /// Open a FerrumDB database at the given path.
    ///
    /// Options (optional):
    /// - `encryptionKey` — a 32-character string for AES-256-GCM encryption at rest.
    ///
    /// ```js
    /// const db = FerrumDB.open("myapp.db");
    /// const encrypted = FerrumDB.open("secure.db", { encryptionKey: "my_super_secret_key_32_bytes_!!?" });
    /// ```
    #[napi(factory)]
    pub fn open(path: String, options: Option<serde_json::Value>) -> Result<Self> {
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| Error::from_reason(e.to_string()))?;

        let encryption_key = options
            .as_ref()
            .and_then(|o| o.get("encryptionKey"))
            .and_then(|v| v.as_str())
            .map(|s| {
                let bytes = s.as_bytes();
                if bytes.len() != 32 {
                    return Err(Error::from_reason(
                        "encryptionKey must be exactly 32 bytes (characters)".to_string(),
                    ));
                }
                let mut key = [0u8; 32];
                key.copy_from_slice(bytes);
                Ok(key)
            })
            .transpose()?;

        let engine = rt.block_on(async {
            let mut fs: Box<dyn AsyncFileSystem> = Box::new(DiskFileSystem);
            if let Some(key) = encryption_key {
                fs = Box::new(EncryptedFileSystem::new(fs, key));
            }
            StorageEngine::with_fs(&path, fs).await
        }).map_err(|e| Error::from_reason(e.to_string()))?;

        Ok(FerrumDB {
            engine: Arc::new(engine),
            rt: Arc::new(rt),
        })
    }

    /// Get a value by key. Returns the value or null if not found.
    ///
    /// ```js
    /// const val = db.get("user:1");
    /// ```
    #[napi]
    pub fn get(&self, env: Env, key: String) -> Result<JsUnknown> {
        let val = self.rt.block_on(self.engine.get(&key));
        match val {
            Some(v) => value_to_js(&env, &v),
            None => env.get_null().map(|n| n.into_unknown()),
        }
    }

    /// Set a key to a JSON-serializable value.
    ///
    /// ```js
    /// db.set("user:1", { name: "alice", role: "admin" });
    /// db.set("counter", 42);
    /// ```
    #[napi]
    pub fn set(&self, env: Env, key: String, value: JsUnknown) -> Result<()> {
        let json_val = js_to_value(&env, value)?;
        self.rt.block_on(self.engine.set(key, json_val))
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(())
    }

    /// Set a key with a TTL (time-to-live) in seconds. The key auto-expires after the given duration.
    ///
    /// ```js
    /// db.setEx("session:abc", { token: "xyz" }, 60); // expires in 60 seconds
    /// ```
    #[napi(js_name = "setEx")]
    pub fn set_ex(&self, env: Env, key: String, value: JsUnknown, ttl_seconds: f64) -> Result<()> {
        let json_val = js_to_value(&env, value)?;
        let ttl = Duration::from_secs_f64(ttl_seconds);
        self.rt.block_on(self.engine.set_ex(key, json_val, Some(ttl)))
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(())
    }

    /// Delete a key. Returns true if it existed, false if not found.
    ///
    /// ```js
    /// const deleted = db.delete("user:1");
    /// ```
    #[napi]
    pub fn delete(&self, key: String) -> Result<bool> {
        let result = self.rt.block_on(self.engine.delete(&key))
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(result.is_some())
    }

    /// List all existing keys in the database.
    ///
    /// ```js
    /// const keys = db.keys();
    /// ```
    #[napi]
    pub fn keys(&self) -> Vec<String> {
        self.rt.block_on(self.engine.keys())
    }

    /// Return the total number of entries.
    ///
    /// ```js
    /// const count = db.count();
    /// ```
    #[napi]
    pub fn count(&self) -> u32 {
        self.rt.block_on(self.engine.len()) as u32
    }

    /// Launch Ferrum Studio web dashboard on the given port.
    /// Runs in the background — your app continues normally.
    ///
    /// ```js
    /// db.startStudio(7474); // opens http://localhost:7474
    /// ```
    #[napi(js_name = "startStudio")]
    pub fn start_studio(&self, port: Option<u32>) -> Result<()> {
        let p = port.unwrap_or(7474) as u16;
        let engine = Arc::clone(&self.engine);
        self.rt.block_on(::ferrumdb::studio::serve(engine, p));
        println!("\x1b[38;5;208m🔥 Ferrum Studio → http://localhost:{}\x1b[0m", p);
        Ok(())
    }

    /// Create a secondary index on a specific JSON field.
    ///
    /// ```js
    /// db.createIndex("role");
    /// ```
    #[napi(js_name = "createIndex")]
    pub fn create_index(&self, field: String) -> Result<()> {
        self.rt.block_on(self.engine.create_index(&field))
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Find all keys where the given field matches the given value.
    ///
    /// ```js
    /// db.createIndex("role");
    /// const admins = db.find("role", "admin");
    /// ```
    #[napi]
    pub fn find(&self, field: String, value: String) -> Result<Vec<String>> {
        let json_val: Value = serde_json::from_str(&value)
            .unwrap_or(Value::String(value.clone()));
        Ok(self.rt.block_on(self.engine.get_by_index(&field, &json_val)))
    }

    /// Commit a transaction atomically.
    ///
    /// ```js
    /// const tx = new Transaction();
    /// tx.set("key1", { value: 1 });
    /// tx.set("key2", { value: 2 });
    /// db.commit(tx);
    /// ```
    #[napi]
    pub fn commit(&self, tx: &Transaction) -> Result<()> {
        let ops: Vec<::ferrumdb::storage::LogOp> = tx.ops.iter().map(|op| {
            match op {
                TxOp::Set { key, value, ttl_seconds } => {
                    let expiry = ttl_seconds.map(|s| {
                        std::time::SystemTime::now() + Duration::from_secs_f64(s)
                    });
                    ::ferrumdb::storage::LogOp::Set {
                        key: key.clone(),
                        value: value.clone(),
                        expiry,
                    }
                }
                TxOp::Delete { key } => {
                    ::ferrumdb::storage::LogOp::Delete { key: key.clone() }
                }
            }
        }).collect();

        self.rt.block_on(self.engine.commit_transaction(ops))
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(())
    }
}

/// Internal transaction operation.
enum TxOp {
    Set { key: String, value: Value, ttl_seconds: Option<f64> },
    Delete { key: String },
}

/// A transaction builder. Use with `db.commit(tx)`.
#[napi]
pub struct Transaction {
    ops: Vec<TxOp>,
}

#[napi]
impl Transaction {
    /// Create a new empty transaction.
    #[napi(constructor)]
    pub fn new() -> Self {
        Transaction { ops: Vec::new() }
    }

    /// Stage a SET operation.
    #[napi]
    pub fn set(&mut self, env: Env, key: String, value: JsUnknown) -> Result<()> {
        let json_val = js_to_value(&env, value)?;
        self.ops.push(TxOp::Set { key, value: json_val, ttl_seconds: None });
        Ok(())
    }

    /// Stage a SET operation with TTL in seconds.
    ///
    /// ```js
    /// tx.setEx("session:abc", { token: "xyz" }, 60);
    /// ```
    #[napi(js_name = "setEx")]
    pub fn set_ex(&mut self, env: Env, key: String, value: JsUnknown, ttl_seconds: f64) -> Result<()> {
        let json_val = js_to_value(&env, value)?;
        self.ops.push(TxOp::Set { key, value: json_val, ttl_seconds: Some(ttl_seconds) });
        Ok(())
    }

    /// Stage a DELETE operation.
    #[napi]
    pub fn delete(&mut self, key: String) {
        self.ops.push(TxOp::Delete { key });
    }
}
