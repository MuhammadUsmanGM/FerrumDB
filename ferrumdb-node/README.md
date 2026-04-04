# FerrumDB Node.js

High-performance, embedded JSON database for Node.js. Powered by Rust and NAPI-RS. Zero-config, ACID-compliant, and built for speed.

<p align="center">
  <img src="https://img.shields.io/badge/Node.js-339933?style=for-the-badge&logo=nodedotjs&logoColor=white" />
  <img src="https://img.shields.io/badge/Rust-000000?style=for-the-badge&logo=rust&logoColor=white" />
  <img src="https://img.shields.io/badge/License-MIT-green.svg?style=for-the-badge" />
  <img src="https://img.shields.io/badge/Performance-Ultra--Fast-orange?style=for-the-badge" />
</p>

---

## 🚀 Why FerrumDB?

FerrumDB is an embedded key-value database engine written from scratch in Rust. It's designed for applications that need fast local persistence without the overhead of a server process.

- ⚡ **Native Performance**: Built in Rust for O(1) reads and writes.
- 📄 **Native JSON Support**: Store and query structured documents.
- 🔍 **Secondary Indexing**: Query by JSON fields with dedicated indexes.
- ⚛️ **Atomic Transactions**: Batch operations that succeed or fail together.
- 🔐 **Encrypted & Durable**: AES-256-GCM encryption at rest.

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
console.log(db.get("user:1")); 

// 3. Secondary Indexing
db.createIndex("role");
const admins = db.find("role", '"admin"'); 
console.log("Admins:", admins);

// 4. Atomic Transactions
const tx = new Transaction();
tx.set("user:2", { name: "Bob", role: "user" });
db.commit(tx);

// 5. Utility
console.log("Total entries:", db.count());
console.log("All keys:", db.keys());
```

---

## 🛠️ API Reference

### FerrumDb

| Method | Description |
|---|---|
| `static open(path: string): FerrumDb` | Opens or creates a database. |
| `get(key: string): any` | Retrieves a value by key. |
| `set(key: string, value: any): void` | Stores a value. |
| `delete(key: string): boolean` | Deletes a key. |
| `keys(): string[]` | Returns all keys. |
| `count(): number` | Total entries. |

---

## 🖥️ Ferrum Studio (Web Dashboard)

Browse your database visually using the standalone CLI:

```bash
cargo install ferrumdb-cli
ferrumdb web myapp.db
# 🔥 http://localhost:7474
```

---

## 📝 License

MIT — [Muhammad Usman](https://github.com/MuhammadUsmanGM)
