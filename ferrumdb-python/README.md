# FerrumDB Python Bindings

**FerrumDB** is a zero-setup embedded document database for Python, powered by Rust.

- 🔥 **Zero-setup** — no server, no config, just `open()`
- ⚡ **Rust-speed** — AOF writes, in-memory index for O(1) reads
- 🔐 **AES-256 Encrypted** — optional encryption at rest
- 📄 **Native JSON** — store any structured document
- 🔍 **Secondary Indexing** — query by JSON fields

## Installation

### From PyPI (recommended)

```bash
pip install ferrumdb
```

### From source (requires Rust toolchain + maturin)

```bash
# Install maturin
pip install maturin

# Clone and build
git clone https://github.com/MuhammadUsmanGM/FerrumDB.git
cd FerrumDB/ferrumdb-python
maturin develop --release
```

## Quick Start

```python
from ferrumdb import FerrumDB

# Zero-setup: creates myapp.db if it doesn't exist
db = FerrumDB.open("myapp.db")

# Store any JSON-serializable value
db.set("user:1", {"name": "alice", "role": "admin", "score": 99})
db.set("user:2", {"name": "bob", "role": "user", "score": 45})
db.set("counter", 42)

# Read back
user = db.get("user:1")
print(user)  # {"name": "alice", "role": "admin", "score": 99}

print(db.count())  # 3
print(db.keys())   # ["user:1", "user:2", "counter"]

# Delete
db.delete("counter")
```

## Secondary Indexing

Query by JSON field values in O(1) time:

```python
from ferrumdb import FerrumDB

db = FerrumDB.open("myapp.db")

# Add data
db.set("user:1", {"name": "alice", "role": "admin"})
db.set("user:2", {"name": "bob", "role": "user"})
db.set("user:3", {"name": "charlie", "role": "admin"})

# Create index on 'role' field
db.create_index("role")

# Query by indexed field
admins = db.find("role", '"admin"')
print(admins)  # ["user:1", "user:3"]
```

## API Reference

| Method | Description |
|--------|-------------|
| `FerrumDB.open(path)` | Open/create database at path |
| `db.set(key, value)` | Store any JSON-serializable value |
| `db.get(key)` | Retrieve value (returns `None` if not found) |
| `db.delete(key)` | Delete a key (returns `True` if existed) |
| `db.keys()` | List all keys |
| `db.count()` | Total number of entries |
| `db.create_index(field)` | Build secondary index on JSON field |
| `db.find(field, value)` | Query by indexed field (value as JSON string) |

## Limitations

FerrumDB makes specific trade-offs for simplicity and performance:

- **Entire index in RAM** — Best for databases <1GB
- **Single-writer only** — One process per database file
- **No range queries** — Only exact value matches on indexed fields
- **No nested field indexes** — Only top-level JSON fields supported

See [GitHub](https://github.com/MuhammadUsmanGM/FerrumDB) for full documentation.

## License

MIT — See LICENSE for details.
