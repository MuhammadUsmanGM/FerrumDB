# FerrumDB Python Bindings

High-performance embedded document database for Python, powered by Rust.

## Installation

**From source (requires Rust toolchain + maturin):**
```bash
pip install maturin
cd ferrumdb-python
maturin develop
```

**Why FerrumDB?**
- 🔥 **Zero-setup** — no server, no config, just `open()` 
- ⚡ **Rust-speed** — AOF writes, in-memory index for O(1) reads
- 🔐 **Encrypted** — AES-256 GCM encryption at rest
- 📄 **Document DB** — native JSON, secondary indexing

## Usage

```python
from ferrumdb import FerrumDB

# Open (creates if not exists)
db = FerrumDB.open("myapp.db")

# Basic operations
db.set("user:1", {"name": "alice", "role": "admin", "score": 99})
db.set("user:2", {"name": "bob", "role": "user", "score": 45})
db.set("counter", 42)
db.set("greeting", "hello world")

val = db.get("user:1")   # => '{"name":"alice","role":"admin","score":99}'
count = db.count()       # => 4
keys = db.keys()         # => ["user:1", "user:2", "counter", "greeting"]
deleted = db.delete("counter")  # => True

# Secondary Indexing (O(1) field lookups)
db.create_index("role")
admins = db.find("role", '"admin"')  # => ["user:1"]
```

## API Reference

| Method | Description |
|--|--|
| `FerrumDB.open(path)` | Open/create database at path |
| `db.set(key, value)` | Store any JSON-serializable value |
| `db.get(key)` | Retrieve value, returns `None` if not found |
| `db.delete(key)` | Delete a key, returns `True` if it existed |
| `db.keys()` | List all keys |
| `db.count()` | Total number of entries |
| `db.create_index(field)` | Build secondary index on JSON field |
| `db.find(field, value)` | Query by indexed field |
