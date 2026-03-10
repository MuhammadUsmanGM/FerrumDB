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
//! db.create_index("role")
//! admins = db.find("role", "admin")  # => ["user:1"]
//! ```

use pyo3::prelude::*;
use pyo3::exceptions::{PyRuntimeError, PyKeyError};
use std::sync::Arc;
use ferrumdb::{StorageEngine, Transaction, Config, FerrumDB as RustDB};
use serde_json::Value;

/// Convert a JSON Value to a Python object
fn value_to_py(py: Python<'_>, val: Value) -> PyObject {
    match val {
        Value::Null => py.None(),
        Value::Bool(b) => b.into_pyobject(py).unwrap().into_any().unbind(),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                i.into_pyobject(py).unwrap().into_any().unbind()
            } else if let Some(f) = n.as_f64() {
                f.into_pyobject(py).unwrap().into_any().unbind()
            } else {
                n.to_string().into_pyobject(py).unwrap().into_any().unbind()
            }
        }
        Value::String(s) => s.into_pyobject(py).unwrap().into_any().unbind(),
        Value::Array(arr) => {
            let list: Vec<PyObject> = arr.into_iter().map(|v| value_to_py(py, v)).collect();
            list.into_pyobject(py).unwrap().into_any().unbind()
        }
        Value::Object(_) => {
            // Serialize objects as JSON strings by default; use json.loads in Python
            val.to_string().into_pyobject(py).unwrap().into_any().unbind()
        }
    }
}

/// Convert a Python object to a JSON Value
fn py_to_value(obj: &Bound<'_, PyAny>) -> PyResult<Value> {
    // Try to get string repr and parse as JSON
    let s = obj.str()?.to_string();
    serde_json::from_str(&s).map_err(|_| {
        // Fallback: treat as a plain string value
        Ok::<Value, ()>(Value::String(s.clone()))
    }).unwrap_or_else(|_| Value::String(s))
    .into_py_result_ok()
}

trait IntoPyResultOk<T> {
    fn into_py_result_ok(self) -> PyResult<T>;
}

impl<T> IntoPyResultOk<T> for T {
    fn into_py_result_ok(self) -> PyResult<T> {
        Ok(self)
    }
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
    /// ```python
    /// db = FerrumDB.open("myapp.db")
    /// ```
    #[staticmethod]
    pub fn open(path: &str) -> PyResult<Self> {
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;

        let engine = rt.block_on(StorageEngine::new(path))
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;

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
    pub fn set(&self, py: Python<'_>, key: String, value: Bound<'_, PyAny>) -> PyResult<()> {
        let json_val = py_to_value(&value)?;
        self.rt.block_on(self.engine.set(key, json_val))
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
}

/// A transaction builder. Use with `db.commit()`.
#[pyclass(name = "Transaction")]
pub struct PyTransaction {
    ops: Vec<(String, Option<Value>)>, // (key, Some(value)) = set, (key, None) = delete
}

#[pymethods]
impl PyTransaction {
    #[new]
    pub fn new() -> Self {
        PyTransaction { ops: Vec::new() }
    }

    /// Stage a SET operation.
    pub fn set(&mut self, py: Python<'_>, key: String, value: Bound<'_, PyAny>) -> PyResult<()> {
        let json_val = py_to_value(&value)?;
        self.ops.push((key, Some(json_val)));
        Ok(())
    }

    /// Stage a DELETE operation.
    pub fn delete(&mut self, key: String) {
        self.ops.push((key, None));
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
    m.add("__version__", "0.1.0")?;
    m.add("__author__", "FerrumDB Team")?;
    Ok(())
}
