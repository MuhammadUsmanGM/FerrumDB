# FerrumDB Python Bindings

[![PyPI](https://img.shields.io/pypi/v/ferrumdb.svg)](https://pypi.org/project/ferrumdb/)
[![PyPI Downloads](https://img.shields.io/pypi/dm/ferrumdb.svg)](https://pypi.org/project/ferrumdb/)
[![Python Versions](https://img.shields.io/pypi/pyversions/ferrumdb.svg)](https://pypi.org/project/ferrumdb/)

**FerrumDB** is a zero-setup embedded document database for Python, powered by Rust.

- **Zero-setup** — no server, no config, just `open()`
- **Rust-speed** — AOF writes, in-memory index for O(1) reads
- **AES-256 Encrypted** — optional encryption at rest
- **Native JSON** — store any structured document
- **Secondary Indexing** — query by JSON fields
- **TTL Support** — keys auto-expire after a configurable duration
- **Built-in Dashboard** — launch Ferrum Studio from your app

## Installation

### From PyPI (recommended)

```bash
pip install ferrumdb
```

### From source (requires Rust toolchain + maturin)

```bash
pip install maturin
git clone https://github.com/MuhammadUsmanGM/FerrumDB.git
cd FerrumDB/ferrumdb-python
maturin develop --release
```

## Quick Start

```python
from ferrumdb import FerrumDB, Transaction

# Zero-setup: creates myapp.db if it doesn't exist
db = FerrumDB.open("myapp.db")

# Store any JSON-serializable value
db.set("user:1", {"name": "alice", "role": "admin", "score": 99})
db.set("user:2", {"name": "bob", "role": "user", "score": 45})

# TTL — auto-expires after 60 seconds
db.set_ex("session:abc", {"token": "xyz"}, 60)

# Read back
user = db.get("user:1")
print(user)  # {"name": "alice", "role": "admin", "score": 99}

print(db.count())  # 2
print(db.keys())   # ["user:1", "user:2"]

# Delete
db.delete("user:2")
```

## Encryption

Open a database with AES-256-GCM encryption at rest:

```python
db = FerrumDB.open("secure.db", encryption_key="my_super_secret_key_32_bytes_!!?")
db.set("secret", {"classified": True})
```

The key must be exactly 32 characters.

## Secondary Indexing

Query by JSON field values in O(1) time:

```python
db = FerrumDB.open("myapp.db")

db.set("user:1", {"name": "alice", "role": "admin"})
db.set("user:2", {"name": "bob", "role": "user"})
db.set("user:3", {"name": "charlie", "role": "admin"})

db.create_index("role")
admins = db.find("role", '"admin"')
print(admins)  # ["user:1", "user:3"]
```

## Transactions

```python
tx = Transaction()
tx.set("key1", {"value": 1})
tx.set_ex("cache:temp", {"data": 123}, 300)  # TTL in transactions too
tx.delete("old_key")
db.commit(tx)  # all-or-nothing
```

## Ferrum Studio (Web Dashboard)

Launch the built-in web dashboard directly from your app:

```python
db = FerrumDB.open("myapp.db")
db.start_studio(7474)  # http://localhost:7474
```

Browse keys, inspect values, set/delete entries, and view real-time metrics — no extra tools needed.

## API Reference

| Method | Description |
|--------|-------------|
| `FerrumDB.open(path, encryption_key=None)` | Open/create database. Optional AES-256 encryption. |
| `db.set(key, value)` | Store any JSON-serializable value |
| `db.set_ex(key, value, ttl_seconds)` | Store with TTL (auto-expires) |
| `db.get(key)` | Retrieve value (returns `None` if not found) |
| `db.delete(key)` | Delete a key (returns `True` if existed) |
| `db.keys()` | List all keys |
| `db.count()` | Total number of entries |
| `db.create_index(field)` | Build secondary index on JSON field |
| `db.find(field, value)` | Query by indexed field (value as JSON string) |
| `db.commit(tx)` | Commit an atomic transaction |
| `db.start_studio(port=7474)` | Launch Ferrum Studio dashboard |

### Transaction

| Method | Description |
|--------|-------------|
| `Transaction()` | Create a new transaction |
| `tx.set(key, value)` | Stage a SET operation |
| `tx.set_ex(key, value, ttl_seconds)` | Stage a SET with TTL |
| `tx.delete(key)` | Stage a DELETE operation |

## Limitations

- **Entire index in RAM** — Best for databases <1GB
- **Single-writer only** — One process per database file
- **No range queries** — Only exact value matches on indexed fields
- **No nested field indexes** — Only top-level JSON fields supported

See [GitHub](https://github.com/MuhammadUsmanGM/FerrumDB) for full documentation.

## License

MIT — See LICENSE for details.
