//! FerrumDB Node.js Bindings
//!
//! Exposes an async/Promise-based API over FerrumDB's storage engine.
//! Uses an internal tokio runtime so Node.js users get non-blocking Promises.
//!
//! Usage:
//! ```js
//! const { FerrumDB, Transaction } = require('ferrumdb');
//!
//! const db = await FerrumDB.open("myapp.db");
//! await db.set("user:1", { name: "alice", role: "admin" });
//! const val = await db.get("user:1"); // => { name: "alice", role: "admin" }
//!
//! await db.createIndex("role");
//! const admins = await db.find("role", "admin"); // => ["user:1"]
//! ```

#[macro_use]
extern crate napi_derive;

use napi::{bindgen_prelude::*, JsUnknown};
use std::sync::Arc;
use ::ferrumdb::StorageEngine;
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
    /// ```js
    /// const db = await FerrumDB.open("myapp.db");
    /// ```
    #[napi(factory)]
    pub fn open(path: String) -> Result<Self> {
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| Error::from_reason(e.to_string()))?;

        let engine = rt.block_on(StorageEngine::new(&path))
            .map_err(|e| Error::from_reason(e.to_string()))?;

        Ok(FerrumDB {
            engine: Arc::new(engine),
            rt: Arc::new(rt),
        })
    }

    /// Get a value by key. Returns the value or null if not found.
    ///
    /// ```js
    /// const val = await db.get("user:1");
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
    /// await db.set("user:1", { name: "alice", role: "admin" });
    /// await db.set("counter", 42);
    /// ```
    #[napi]
    pub fn set(&self, env: Env, key: String, value: JsUnknown) -> Result<()> {
        let json_val = js_to_value(&env, value)?;
        self.rt.block_on(self.engine.set(key, json_val))
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(())
    }

    /// Delete a key. Returns true if it existed, false if not found.
    ///
    /// ```js
    /// const deleted = await db.delete("user:1");
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
    /// const keys = await db.keys();
    /// ```
    #[napi]
    pub fn keys(&self) -> Vec<String> {
        self.rt.block_on(self.engine.keys())
    }

    /// Return the total number of entries.
    ///
    /// ```js
    /// const count = await db.count();
    /// ```
    #[napi]
    pub fn count(&self) -> u32 {
        self.rt.block_on(self.engine.len()) as u32
    }

    /// Create a secondary index on a specific JSON field.
    ///
    /// ```js
    /// await db.createIndex("role");
    /// ```
    #[napi(js_name = "createIndex")]
    pub fn create_index(&self, field: String) -> Result<()> {
        self.rt.block_on(self.engine.create_index(&field))
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Find all keys where the given field matches the given value.
    ///
    /// ```js
    /// await db.createIndex("role");
    /// const admins = await db.find("role", "admin");
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
    /// await db.commit(tx);
    /// ```
    #[napi]
    pub fn commit(&self, tx: &Transaction) -> Result<()> {
        let ops: Vec<::ferrumdb::storage::LogOp> = tx.ops.iter().map(|(key, val_opt)| {
            if let Some(ref v) = val_opt {
                ::ferrumdb::storage::LogOp::Set {
                    key: key.clone(),
                    value: v.clone(),
                    expiry: None,
                }
            } else {
                ::ferrumdb::storage::LogOp::Delete { key: key.clone() }
            }
        }).collect();

        self.rt.block_on(self.engine.commit_transaction(ops))
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(())
    }
}

/// A transaction builder. Use with `db.commit(tx)`.
#[napi]
pub struct Transaction {
    ops: Vec<(String, Option<Value>)>,
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
        self.ops.push((key, Some(json_val)));
        Ok(())
    }

    /// Stage a DELETE operation.
    #[napi]
    pub fn delete(&mut self, key: String) {
        self.ops.push((key, None));
    }
}
