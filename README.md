# 🛡️ FerrumDB

<p align="center">
  <img src="https://img.shields.io/badge/Rust-000000?style=for-the-badge&logo=rust&logoColor=white" />
  <img src="https://img.shields.io/badge/License-MIT-green.svg?style=for-the-badge" />
  <img src="https://img.shields.io/badge/Version-0.1.3-blue.svg?style=for-the-badge" />
  <img src="https://img.shields.io/badge/Storage-Append--Only-orange?style=for-the-badge" />
</p>

---

**FerrumDB** is a premium, high-performance local key-value database built in **Rust**. Designed for developers who need a reliable, "zero-setup" structured data store that feels as fast as a cache but is as durable as a disk-backed database.

## 🌟 Why FerrumDB?

- ⚡ **Append-Only Architecture (AOF)**: $O(1)$ write performance. We never overwrite—we only grow.
- 📦 **Structured Data**: Native support for **JSON Values**. Store objects, arrays, and numbers directly.
- ⏳ **Time-To-Live (TTL)**: Built-in data expiration for efficient local caching.
- 🧹 **Background Compaction**: Automatic garbage collection to keep your storage footprint minimal.
- 🏗️ **Embeddable Library**: Use it as a CLI tool or import it as a high-level Rust crate.
- 🛡️ **Crash Resilient**: Atomic swaps and `sync_data` guarantee your data survives power loss.

---

## 🚀 Quick Start

### Library Usage (Zero-Setup)

Add FerrumDB to your project and start storing data in seconds:

```rust
use ferrumdb::FerrumDB;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Zero-setup open (defaults to ./ferrum.db)
    let db = FerrumDB::open_default().await?;

    // Store structured JSON
    db.set("user:1".into(), json!({
        "name": "Usman",
        "role": "Premium Developer"
    })).await?;

    // Retrieve data
    if let Some(user) = db.get("user:1").await {
        println!("User: {}", user["name"]);
    }

    Ok(())
}
```

### CLI Interactive REPL

Experience the premium terminal interface with autocomplete and syntax highlighting:

```bash
cargo run --release
```

---

## 🛠️ Commands Reference

| Command | Usage | Description |
| :--- | :--- | :--- |
| **SET** | `SET <key> <json>` | Store structured data (Strings, Objects, Arrays) |
| **GET** | `GET <key>` | Retrieve and pretty-print stored data |
| **DELETE** | `DELETE <key>` | Remove a key-value pair |
| **KEYS** | `KEYS` | List all indexed keys |
| **COUNT** | `COUNT` | Show total number of entries |
| **COMPACT**| `COMPACT` | Manually trigger log file optimization |
| **HELP** | `HELP` | Show commands and session metrics |

---

## 🎨 Premium REPL Features

- **Tab-Complete**: Instantly complete commands and keys.
- **Colorized Output**: High-contrast, easy-to-read terminal feedback.
- **JSON Pretty-Print**: Structured output for complex data.

---

## 📐 Architecture

- **Engine**: Bitcask-lite inspired log-structured storage.
- **Index**: In-memory `HashMap` leveraging `tokio::sync::RwLock` for high concurrency.
- **Persistence**: Binary serialization via `bincode` for maximum speed and minimal disk usage.

---

## 📝 License

Distributed under the **MIT License**. See `LICENSE` for more information.

---

<p align="center">
  Built with 🦀 by Muhammad Usman
</p>
