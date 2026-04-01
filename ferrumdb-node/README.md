# ⚡ FerrumDB Node.js

<p align="center">
  <img src="https://img.shields.io/badge/Node.js-339933?style=for-the-badge&logo=nodedotjs&logoColor=white" />
  <img src="https://img.shields.io/badge/Rust-000000?style=for-the-badge&logo=rust&logoColor=white" />
  <img src="https://img.shields.io/badge/License-MIT-green.svg?style=for-the-badge" />
  <img src="https://img.shields.io/badge/Performance-Ultra--Fast-orange?style=for-the-badge" />
</p>

<p align="center">
  <strong>The high-performance, embedded JSON database for Node.js.</strong><br/>
  Powered by Rust and NAPI-RS. Zero-config, ACID-compliant, and built for speed.
</p>

---

## 🚀 Why FerrumDB?

FerrumDB is an embedded key-value database engine written from scratch in Rust. It's designed for applications that need fast local persistence without the overhead of a server process.

- ⚡ **Native Performance**: Built in Rust for O(1) reads and writes.
- 📄 **Native JSON Support**: Store and query structured documents.
- 🔍 **Secondary Indexing**: Query by JSON fields with dedicated indexes.
- ⚛️ **Atomic Transactions**: Batch operations that succeed or fail together.
- 🔐 **Encrypted & Durable**: AES-256-GCM encryption at rest and configurable fsync policies.

---

## 📦 Installation

```bash
npm install ferrumdb
```

---

## 💡 Quick Start

```javascript
const { FerrumDb, Transaction } = require('ferrumdb');

// 1. Open (or create) a database
const db = FerrumDb.open("myapp.db");

// 2. Simple CRUD
db.set("user:1", { name: "Alice", role: "admin", points: 100 });
console.log(db.get("user:1")); // => { name: "Alice", role: "admin", points: 100 }

// 3. Secondary Indexing
db.createIndex("role");
const admins = db.find("role", '"admin"'); // Query by index
console.log("Admins:", admins); // => ["user:1"]

// 4. Atomic Transactions
const tx = new Transaction();
tx.set("user:2", { name: "Bob", role: "user" });
tx.delete("legacy-key");
db.commit(tx);

// 5. Utility
console.log("Total entries:", db.count());
console.log("All keys:", db.keys());
```

---

## 🛠️ API Reference

### `FerrumDb`

| Method | Description |
|---|---|
| `static open(path: string): FerrumDb` | Opens or creates a database at the specified path. |
| `get(key: string): any` | Retrieves a value by key. Returns `null` if not found. |
| `set(key: string, value: any): void` | Stores a JSON-serializable value. |
| `delete(key: string): boolean` | Deletes a key. Returns `true` if it existed. |
| `keys(): string[]` | Returns an array of all keys in the database. |
| `count(): number` | Returns the total number of entries. |
| `createIndex(field: string): void` | Creates a secondary index on a specific JSON field. |
| `find(field: string, value: string): string[]` | Finds keys where the field matches the value (value must be a JSON string). |
| `commit(tx: Transaction): void` | Commits an atomic transaction. |

### `Transaction`

| Method | Description |
|---|---|
| `set(key: string, value: any): void` | Stages a SET operation. |
| `delete(key: string): void` | Stages a DELETE operation. |

---

## ⚙️ Advanced Configuration

FerrumDB uses environment variables for global engine settings:

- `FERRUMDB_FSYNC`: 
  - `always`: Sync every write (maximum durability).
  - `never`: Let OS handle sync (maximum performance).
  - `periodic:200`: Sync every 200ms (balanced).

---

## ⚠️ Important Note

The current Node.js bindings use a synchronous API powered by an internal Rust runtime. While it offers extreme performance, it executes on the main thread for simple CRUD operations.

---

## 📝 License

MIT — Built with 🦀 by [Muhammad Usman](https://github.com/MuhammadUsmanGM)
