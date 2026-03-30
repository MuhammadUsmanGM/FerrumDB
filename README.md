# ⚡ FerrumDB

<p align="center">
  <img src="https://img.shields.io/badge/Rust-000000?style=for-the-badge&logo=rust&logoColor=white" />
  <img src="https://img.shields.io/badge/Python-3776AB?style=for-the-badge&logo=python&logoColor=white" />
  <img src="https://img.shields.io/badge/Node.js-339933?style=for-the-badge&logo=nodedotjs&logoColor=white" />
  <img src="https://img.shields.io/badge/License-MIT-green.svg?style=for-the-badge" />
  <img src="https://img.shields.io/badge/AES--256-Encrypted-red?style=for-the-badge" />
  <img src="https://img.shields.io/badge/version-0.1.1-blue?style=for-the-badge" />
</p>

<p align="center">
  <strong>A high-performance, embedded document database written from scratch in Rust.</strong><br/>
  No server. No config files. No migrations. Open a file and go.
</p>

---

## What is FerrumDB?

FerrumDB is an embedded key-value database engine built in Rust, designed for applications that need fast local persistence without the overhead of a server process. It is inspired by Bitcask and implements a custom binary log format, in-memory indexing, AES-256-GCM encryption at rest, atomic transactions, and a live web dashboard — all in ~1,000 lines of safe, async Rust.

It ships Python bindings via PyO3 (`pip install ferrumdb`) and Node.js bindings via NAPI-RS (`npm install ferrumdb`).

---

## 🌟 Features

| Feature | Detail |
|---|---|
| ⚡ **O(1) reads & writes** | Append-only log + in-memory `HashMap` index rebuilt on startup |
| 📄 **Native JSON documents** | Store any structured data; values are `serde_json::Value` |
| 🔍 **Secondary indexing** | O(1) field lookups via `create_index()` — maintained live on writes |
| 🔐 **AES-256-GCM encryption** | Per-block encryption with random nonces; data is protected at rest |
| ⚛️ **Atomic transactions** | All-or-nothing batches written as a single log entry |
| ⏱️ **Configurable fsync policy** | `Always` / `Periodic(ms)` / `Never` — tune durability vs. throughput |
| 🖥️ **Ferrum Studio** | Built-in web dashboard (Axum) at `localhost:7474` |
| 🐍 **Python bindings** | `pip install ferrumdb` — no Rust toolchain required |
| 🛡️ **Crash resilience** | Log compaction via atomic `rename()`; incomplete records are skipped |
| 📊 **Observability** | Lock-free atomic metrics: ops/sec, uptime, GET/SET/DELETE counts |

---

## 🏗️ Architecture

FerrumDB was built ground-up without using an existing storage library. Every layer is custom:

```
┌─────────────────────────────────────────┐
│                FerrumDB API              │  ← High-level Rust & Python interface
├─────────────────────────────────────────┤
│             StorageEngine               │  ← Core engine: index + log management
│  ┌─────────────────┐  ┌──────────────┐  │
│  │  In-Memory Index │  │ Secondary    │  │
│  │  HashMap<K,V>   │  │ Indexes      │  │
│  │  RwLock async   │  │ HashMap<F,V> │  │
│  └────────┬────────┘  └──────────────┘  │
│           │ append / reads              │
│  ┌────────▼────────────────────────┐    │
│  │   Append-Only Log (AOF)         │    │  ← Bitcask-inspired binary format
│  │   [len: u64][JSON bytes]...     │    │     length-prefixed, sequential
│  └────────┬────────────────────────┘    │
├───────────┼─────────────────────────────┤
│  ┌────────▼────────────────────────┐    │
│  │  AsyncFileSystem trait          │    │  ← Pluggable I/O abstraction
│  │  ┌──────────┐  ┌─────────────┐  │    │
│  │  │   Disk   │  │  Encrypted  │  │    │  ← Decorator pattern
│  │  │  (tokio) │  │  (AES-GCM)  │  │    │     random nonce per block
│  │  └──────────┘  └─────────────┘  │    │
│  └─────────────────────────────────┘    │
└─────────────────────────────────────────┘
```

**Key design decisions:**

- **Bitcask AOF**: Writes are append-only (fast, sequential I/O). The in-memory index is the source of truth for reads. On startup, the engine replays the log to rebuild state — making recovery deterministic and crash-safe.
- **Pluggable `AsyncFileSystem` trait**: The I/O layer is fully abstracted. `DiskFileSystem` and `EncryptedFileSystem` implement the same trait — swapped via the decorator pattern. This makes the storage engine 100% testable without touching disk.
- **AES-256-GCM per block**: Each binary record is individually encrypted with a cryptographically random 12-byte nonce. The nonce is stored alongside the ciphertext. GCM authentication tags detect any file tampering.
- **Tokio async throughout**: Reads use `RwLock` (many concurrent readers), writes serialize via write lock. Metrics use `AtomicU64` — no lock contention on the hot path.
- **Log compaction**: A background `compact()` rewrites only live (non-expired, non-deleted) records to a temp file, then swaps atomically via `rename()` — POSIX-atomic, no data loss possible.

---

## ⚙️ Technical Stack

| Component | Technology |
|---|---|
| Language | Rust (2021 edition) |
| Async runtime | Tokio |
| Serialization | serde + serde_json |
| Encryption | aes-gcm (AES-256-GCM) |
| Web dashboard | Axum |
| Python bindings | PyO3 (via maturin) |
| Benchmarking | Criterion |
| Testing | tokio::test + tempfile |

---

## 📊 Performance

Benchmarked with [Criterion](https://github.com/bheisler/criterion.rs) on an append-only log with `FsyncPolicy::Never` (max throughput):

| Operation | Performance |
|---|---|
| Single `SET` | ~1–3 µs |
| Single `GET` (in-memory) | < 1 µs |
| 1,000 sequential `SET`s | ~2–5 ms |
| 100 concurrent `SET`s (Tokio tasks) | ~3–8 ms |
| Secondary index query (100 docs) | < 1 µs |

> Run benchmarks yourself: `cargo bench`

---

## 🐍 Python Installation & Usage

FerrumDB is available on PyPI. Install it using pip:

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

---

## 🦀 Rust Installation & Usage

FerrumDB is available on crates.io. Add it to your project:

```bash
cargo add ferrumdb
cargo add tokio -F full
cargo add serde_json
```

Or manually add to your `Cargo.toml`:

```toml
[dependencies]
ferrumdb = "0.1.1"
tokio = { version = "1", features = ["full"] }
serde_json = "1"
```

```rust
use ferrumdb::{FerrumDB, Config, Transaction, FsyncPolicy};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Standard open (zero-setup, uses ferrum.db)
    let db = FerrumDB::open_default().await?;

    // Store documents
    db.set("user:1".into(), json!({"name": "alice", "role": "admin"})).await?;

    // Secondary index query
    db.create_index("role").await?;
    let admins = db.find("role", &json!("admin")).await;

    // Atomic transaction
    let tx = Transaction::new()
        .set("k1".into(), json!({"tag": "blue"}))
        .set("k2".into(), json!({"tag": "red"}))
        .delete("k1".into());
    db.commit(tx).await?;

    // Encrypted database (AES-256-GCM, random nonce per block)
    let key: [u8; 32] = *b"my_super_secret_key_32_bytes_!!?";
    let db_enc = FerrumDB::open(
        Config::new()
            .with_encryption(key)
            .with_fsync_policy(FsyncPolicy::Periodic(std::time::Duration::from_millis(100)))
    ).await?;

    Ok(())
}
```

---

## 🖥️ Ferrum Studio

When you run the REPL, Ferrum Studio auto-launches — an embedded web dashboard to browse, query, and inspect your live database, including real-time operation metrics.

```bash
cargo run --release
# 🔥 Ferrum Studio → http://localhost:7474
```

---

## 🖥️ CLI REPL

```bash
cargo run
cargo run -- --fsync=always   # strongest durability
```

| Command | Description |
|---|---|
| `SET <key> <json>` | Store a document |
| `GET <key>` | Retrieve and pretty-print |
| `DELETE <key>` | Remove a key |
| `KEYS` | List all keys |
| `COUNT` | Total number of entries |
| `INDEX <field>` | Create secondary index on JSON field |
| `FIND <field> <value>` | Query by indexed field |
| `HELP` | Show commands + live session metrics |

---

## 📂 Examples

Full working examples for each language are in the [`examples/`](./examples) directory:

| Example | Language | Description | Run |
|---|---|---|---|
| [**rust-example**](./examples/rust-example) | Rust | Task Manager — CRUD, secondary indexes, transactions, TTL | `cd examples/rust-example && cargo run` |
| [**python-example**](./examples/python-example) | Python | Contact Book — CRUD, secondary indexes, transactions | `cd examples/python-example && python main.py` |
| [**node-example**](./examples/node-example) | Node.js | Note Taker — CRUD, secondary indexes, transactions | `cd examples/node-example && node main.mjs` |

Each example is self-contained and demonstrates the core FerrumDB API in its respective language.

---

## ⚠️ Known Limitations

FerrumDB optimizes for simplicity and embedded use cases. Understand the trade-offs:

| Limitation | Reason | Workaround |
|---|---|---|
| **Entire index in RAM** | O(1) reads require full `HashMap` in memory | Best for databases < 1 GB |
| **Single-writer only** | Append-only log has no cross-process lock protocol | One process per DB file |
| **No range queries** | Secondary indexes store exact value matches | Use Tantivy for range scans |
| **No nested field indexes** | Indexes only top-level JSON keys | Flatten documents before storing |
| **Blocking compaction** | Rewrites entire log — hold write lock | Schedule during low-traffic |
| **No WAL / MVCC** | Simpler append-only design | Accept occasional contention |
| **No replication** | Single-file, embedded design | Handle replication at app level |

**Best for:** local-first apps, desktop tools, embedded caching, session/config stores, write-heavy workloads.

**Not for:** large datasets (> 1 GB), complex queries (JOINs, aggregations), multi-writer or distributed scenarios.

---

## Environment Config

```bash
set FERRUMDB_FSYNC=always        # sync every write (safest)
set FERRUMDB_FSYNC=never         # never sync (fastest)
set FERRUMDB_FSYNC=periodic:200  # sync every 200ms
```

```rust
let db = FerrumDB::open_from_env().await?;
```

---

## 📝 License

MIT — see `LICENSE` for details.

<p align="center">Built with 🦀 by <a href="https://github.com/MuhammadUsmanGM">Muhammad Usman</a></p>
