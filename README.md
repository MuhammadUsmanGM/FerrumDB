# ⚡ FerrumDB

<p align="center">
  <img src="https://img.shields.io/badge/Rust-000000?style=for-the-badge&logo=rust&logoColor=white" />
  <img src="https://img.shields.io/badge/Python-3776AB?style=for-the-badge&logo=python&logoColor=white" />
  <img src="https://img.shields.io/badge/License-MIT-green.svg?style=for-the-badge" />
  <img src="https://img.shields.io/badge/AES--256-Encrypted-red?style=for-the-badge" />
</p>

**FerrumDB** is a premium, zero-setup embedded document database for **Rust** and **Python**. No server. No config. No migrations. Just open a file and go.

---

## 🌟 Features

| | |
|---|---|
| ⚡ **O(1) reads & writes** | Append-only log + in-memory HashMap index |
| 📄 **Native JSON** | Store any structured document natively |
| 🔍 **Secondary Indexing** | Query by JSON fields via `create_index()` |
| 🔐 **AES-256 Encryption** | Protect data at rest with one line of config |
| ⚛️ **Atomic Transactions** | All-or-nothing batch operations |
| 🖥️ **Ferrum Studio** | Embedded web dashboard at `localhost:7474` |
| 🐍 **Python Bindings** | `pip install ferrumdb` — no Rust required |
| 🛡️ **Crash Resilient** | `fsync` + atomic rename guarantee durability |

---

## 🐍 Python Usage

```bash
pip install ferrumdb
```

```python
from ferrumdb import FerrumDB

# Zero-setup: creates myapp.db if it doesn't exist
db = FerrumDB.open("myapp.db")

# Store any JSON-serializable value
db.set("user:1", '{"name": "alice", "role": "admin", "score": 99}')
db.set("user:2", '{"name": "bob",   "role": "user",  "score": 45}')

# Read back
print(db.get("user:1"))       # {"name": "alice", "role": "admin", "score": 99}
print(db.count())             # 2
print(db.keys())              # ["user:1", "user:2"]

# Secondary indexing — O(1) field lookups
db.create_index("role")
admins = db.find("role", '"admin"')   # => ["user:1"]

# Delete
db.delete("user:2")
```

Your data is stored in a plain file in your project directory — portable, no server, no Docker.

---

## 🦀 Rust Usage

```toml
# Cargo.toml
[dependencies]
ferrumdb = "0.1.0"
tokio = { version = "1", features = ["full"] }
serde_json = "1"
```

```rust
use ferrumdb::{FerrumDB, Config, Transaction};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Standard open
    let db = FerrumDB::open_default().await?;

    // Store documents
    db.set("user:1".into(), json!({"name": "alice", "role": "admin"})).await?;

    // Query
    db.create_index("role").await?;
    let admins = db.find("role", &json!("admin")).await;

    // Atomic transaction
    let tx = Transaction::new()
        .set("k1".into(), json!({"tag": "blue"}))
        .set("k2".into(), json!({"tag": "red"}))
        .delete("k1".into());
    db.commit(tx).await?;

    // Encrypted database
    let key: [u8; 32] = *b"my_super_secret_key_32_bytes_!!";
    let db_enc = FerrumDB::open(
        Config::new().with_encryption(key)
    ).await?;

    Ok(())
}
```

---

## 🔥 Ferrum Studio

When you run the REPL, Ferrum Studio auto-launches — a premium web dashboard to browse, query, and edit your database visually.

```bash
cargo run --release
# 🔥 Ferrum Studio → http://localhost:7474
```

---

## 🖥️ CLI REPL Commands

```bash
cargo run
```

| Command | Description |
|---|---|
| `SET <key> <json>` | Store a document |
| `GET <key>` | Retrieve and pretty-print |
| `DELETE <key>` | Remove a key |
| `KEYS` | List all keys |
| `COUNT` | Total number of entries |
| `INDEX <field>` | Create secondary index |
| `FIND <field> <value>` | Query by indexed field |
| `HELP` | Show commands + session metrics |

---

## 🏗️ Architecture

- **Storage**: Bitcask-inspired append-only log (AOF)
- **Index**: In-memory `HashMap` with `tokio::sync::RwLock`
- **Encryption**: AES-256-GCM per-block, transparent decorator pattern
- **Compaction**: Atomic log rewrite via temp-file + rename

---

## 📝 License

MIT — see `LICENSE` for details.

<p align="center">Built with 🦀 by Muhammad Usman</p>