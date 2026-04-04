//! FerrumDB Python Bindings
//!
//! Exposes a clean synchronous Python API over FerrumDB's async engine.
//! Uses an internal tokio runtime so Python users never deal with async/await.
//!
//! Usage:
//! ```python
//! from ferrumdb import FerrumDB, Transaction
//!
//! db = FerrumDB.open("myapp.db")
//! db.set("user:1", {"name": "alice", "role": "admin"})
//! val = db.get("user:1")  # => {"name": "alice", "role": "admin"}
//!
//! # With TTL (auto-expires after 60 seconds)
//! db.set_ex("session:abc", {"token": "xyz"}, 60)
//!
//! # With encryption
//! db2 = FerrumDB.open("secure.db", encryption_key="my_super_secret_key_32_bytes_!!?")
//! ```

use pyo3::prelude::*;
use pyo3::exceptions::PyRuntimeError;
use std::sync::Arc;
use std::time::Duration;
use ::ferrumdb::{StorageEngine, DiskFileSystem, EncryptedFileSystem, io::AsyncFileSystem};
use serde_json::Value;

/// Convert a JSON Value to a Python object
fn value_to_py(py: Python<'_>, val: Value) -> PyObject {
    match val {
        Value::Null => py.None(),
        Value::Bool(b) => b.to_object(py),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                i.to_object(py)
            } else if let Some(f) = n.as_f64() {
                f.to_object(py)
            } else {
                n.to_string().to_object(py)
            }
        }
        Value::String(s) => s.to_object(py),
        Value::Array(arr) => {
            let list: Vec<PyObject> = arr.into_iter().map(|v| value_to_py(py, v)).collect();
            list.to_object(py)
        }
        Value::Object(_) => {
            // Serialize objects as JSON strings by default; use json.loads in Python
            val.to_string().to_object(py)
        }
    }
}

/// Convert a Python object to a JSON Value
fn py_to_value(obj: &Bound<'_, PyAny>) -> PyResult<Value> {
    // Try to get string repr and parse as JSON
    let s = obj.str()?.to_string();
    Ok(serde_json::from_str(&s).unwrap_or_else(|_| Value::String(s.clone())))
}

/// The main FerrumDB Python class.
#[pyclass(name = "FerrumDB")]
pub struct PyFerrumDB {
    engine: Arc<StorageEngine>,
    rt: Arc<tokio::runtime::Runtime>,
}

#[pymethods]
impl PyFerrumDB {
    /// Open a FerrumDB database at the given path.
    ///
    /// Args:
    ///     path: Path to the database file.
    ///     encryption_key: Optional 32-character string for AES-256-GCM encryption.
    ///
    /// ```python
    /// db = FerrumDB.open("myapp.db")
    /// encrypted = FerrumDB.open("secure.db", encryption_key="my_super_secret_key_32_bytes_!!?")
    /// ```
    #[staticmethod]
    #[pyo3(signature = (path, encryption_key=None))]
    pub fn open(path: &str, encryption_key: Option<&str>) -> PyResult<Self> {
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;

        let enc_key = encryption_key
            .map(|s| {
                let bytes = s.as_bytes();
                if bytes.len() != 32 {
                    return Err(PyRuntimeError::new_err(
                        "encryption_key must be exactly 32 bytes (characters)",
                    ));
                }
                let mut key = [0u8; 32];
                key.copy_from_slice(bytes);
                Ok(key)
            })
            .transpose()?;

        let engine = rt.block_on(async {
            let mut fs: Box<dyn AsyncFileSystem> = Box::new(DiskFileSystem);
            if let Some(key) = enc_key {
                fs = Box::new(EncryptedFileSystem::new(fs, key));
            }
            StorageEngine::with_fs(path, fs).await
        }).map_err(|e| PyRuntimeError::new_err(e.to_string()))?;

        Ok(PyFerrumDB {
            engine: Arc::new(engine),
            rt: Arc::new(rt),
        })
    }

    /// Set a key to a JSON-serializable value.
    ///
    /// ```python
    /// db.set("user:1", {"name": "alice", "role": "admin"})
    /// db.set("counter", 42)
    /// db.set("greeting", "hello world")
    /// ```
    pub fn set(&self, key: String, value: Bound<'_, PyAny>) -> PyResult<()> {
        let json_val = py_to_value(&value)?;
        self.rt.block_on(self.engine.set(key, json_val))
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
        Ok(())
    }

    /// Set a key with a TTL (time-to-live) in seconds. Auto-expires after the duration.
    ///
    /// ```python
    /// db.set_ex("session:abc", {"token": "xyz"}, 60)  # expires in 60 seconds
    /// ```
    pub fn set_ex(&self, key: String, value: Bound<'_, PyAny>, ttl_seconds: f64) -> PyResult<()> {
        let json_val = py_to_value(&value)?;
        let ttl = Duration::from_secs_f64(ttl_seconds);
        self.rt.block_on(self.engine.set_ex(key, json_val, Some(ttl)))
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
        Ok(())
    }

    /// Get a value by key. Returns None if the key does not exist.
    ///
    /// ```python
    /// val = db.get("user:1")
    /// ```
    pub fn get(&self, py: Python<'_>, key: &str) -> Option<PyObject> {
        let val = self.rt.block_on(self.engine.get(key))?;
        Some(value_to_py(py, val))
    }

    /// Delete a key. Returns True if it existed, False if not found.
    ///
    /// ```python
    /// deleted = db.delete("user:1")
    /// ```
    pub fn delete(&self, key: &str) -> PyResult<bool> {
        let result = self.rt.block_on(self.engine.delete(key))
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
        Ok(result.is_some())
    }

    /// List all existing keys in the database.
    ///
    /// ```python
    /// keys = db.keys()
    /// ```
    pub fn keys(&self) -> Vec<String> {
        self.rt.block_on(self.engine.keys())
    }

    /// Return the total number of entries.
    ///
    /// ```python
    /// count = db.count()
    /// ```
    pub fn count(&self) -> usize {
        self.rt.block_on(self.engine.len())
    }

    /// Create a secondary index on a specific JSON field.
    /// After creation, use `find()` to query by that field.
    ///
    /// ```python
    /// db.create_index("role")
    /// ```
    pub fn create_index(&self, field: &str) -> PyResult<()> {
        self.rt.block_on(self.engine.create_index(field))
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))
    }

    /// Find all keys where the given field matches the given value.
    ///
    /// ```python
    /// db.create_index("role")
    /// admins = db.find("role", "admin")  # => ["user:1", "user:3"]
    /// ```
    pub fn find(&self, field: &str, value: &str) -> PyResult<Vec<String>> {
        // Parse value as JSON for proper type matching
        let json_val: Value = serde_json::from_str(value)
            .unwrap_or(Value::String(value.to_string()));
        Ok(self.rt.block_on(self.engine.get_by_index(field, &json_val)))
    }

    pub fn __repr__(&self) -> String {
        let count = self.rt.block_on(self.engine.len());
        format!("<FerrumDB entries={}>", count)
    }

    /// Commit a transaction atomically.
    ///
    /// ```python
    /// tx = Transaction()
    /// tx.set("key1", {"value": 1})
    /// tx.set("key2", {"value": 2})
    /// db.commit(tx)
    /// ```
    pub fn commit(&self, tx: &mut PyTransaction) -> PyResult<()> {
        let ops: Vec<::ferrumdb::storage::LogOp> = tx.ops.iter().map(|op| {
            match op {
                PyTxOp::Set { key, value, ttl_seconds } => {
                    let expiry = ttl_seconds.map(|s| {
                        std::time::SystemTime::now() + Duration::from_secs_f64(s)
                    });
                    ::ferrumdb::storage::LogOp::Set {
                        key: key.clone(),
                        value: value.clone(),
                        expiry,
                    }
                }
                PyTxOp::Delete { key } => {
                    ::ferrumdb::storage::LogOp::Delete { key: key.clone() }
                }
            }
        }).collect();

        self.rt.block_on(self.engine.commit_transaction(ops))
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
        Ok(())
    }
}

/// Internal transaction operation.
enum PyTxOp {
    Set { key: String, value: Value, ttl_seconds: Option<f64> },
    Delete { key: String },
}

/// A transaction builder. Use with `db.commit()`.
#[pyclass(name = "Transaction")]
pub struct PyTransaction {
    ops: Vec<PyTxOp>,
}

#[pymethods]
impl PyTransaction {
    #[new]
    pub fn new() -> Self {
        PyTransaction { ops: Vec::new() }
    }

    /// Stage a SET operation.
    pub fn set(&mut self, key: String, value: Bound<'_, PyAny>) -> PyResult<()> {
        let json_val = py_to_value(&value)?;
        self.ops.push(PyTxOp::Set { key, value: json_val, ttl_seconds: None });
        Ok(())
    }

    /// Stage a SET operation with TTL in seconds.
    ///
    /// ```python
    /// tx.set_ex("session:abc", {"token": "xyz"}, 60)
    /// ```
    pub fn set_ex(&mut self, key: String, value: Bound<'_, PyAny>, ttl_seconds: f64) -> PyResult<()> {
        let json_val = py_to_value(&value)?;
        self.ops.push(PyTxOp::Set { key, value: json_val, ttl_seconds: Some(ttl_seconds) });
        Ok(())
    }

    /// Stage a DELETE operation.
    pub fn delete(&mut self, key: String) {
        self.ops.push(PyTxOp::Delete { key });
    }

    pub fn __repr__(&self) -> String {
        format!("<Transaction ops={}>", self.ops.len())
    }
}

/// Register the Python module.
#[pymodule]
fn ferrumdb(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyFerrumDB>()?;
    m.add_class::<PyTransaction>()?;
    m.add("__version__", "0.1.2")?;
    m.add("__author__", "FerrumDB Team")?;
    Ok(())
}
