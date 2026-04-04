# FerrumDB Node.js

High-performance, embedded JSON database for Node.js. Powered by Rust and NAPI-RS. Zero-config, ACID-compliant, and built for speed.

<p align="center">
  <img src="https://img.shields.io/badge/Node.js-339933?style=for-the-badge&logo=nodedotjs&logoColor=white" />
  <img src="https://img.shields.io/badge/Rust-000000?style=for-the-badge&logo=rust&logoColor=white" />
  <img src="https://img.shields.io/badge/License-MIT-green.svg?style=for-the-badge" />
  <img src="https://img.shields.io/badge/Performance-Ultra--Fast-orange?style=for-the-badge" />
</p>

---

## Why FerrumDB?

FerrumDB is an embedded key-value database engine written from scratch in Rust. It's designed for applications that need fast local persistence without the overhead of a server process.

- **Native Performance**: Built in Rust for O(1) reads and writes.
- **Native JSON Support**: Store and query structured documents.
- **Secondary Indexing**: Query by JSON fields with dedicated indexes.
- **Atomic Transactions**: Batch operations that succeed or fail together.
- **TTL Support**: Keys auto-expire after a configurable duration.
- **AES-256 Encryption**: Optional encryption at rest.
- **Built-in Dashboard**: Launch Ferrum Studio from your app — no extra tools needed.

---

## Installation

```bash
npm install ferrumdb
```

---

## Quick Start

```javascript
const { FerrumDB, Transaction } = require('ferrumdb');

// 1. Open (or create) a database
const db = FerrumDB.open("myapp.db");

// 2. Simple CRUD
db.set("user:1", { name: "Alice", role: "admin", points: 100 });
console.log(db.get("user:1"));

// 3. TTL — auto-expires after 60 seconds
db.setEx("session:abc", { token: "xyz" }, 60);

// 4. Secondary Indexing
db.createIndex("role");
const admins = db.find("role", '"admin"');
console.log("Admins:", admins);

// 5. Atomic Transactions
const tx = new Transaction();
tx.set("user:2", { name: "Bob", role: "user" });
tx.setEx("cache:temp", { data: 123 }, 300); // TTL in transactions too
db.commit(tx);

// 6. Utility
console.log("Total entries:", db.count());
console.log("All keys:", db.keys());
```

---

## Encryption

Open a database with AES-256-GCM encryption at rest:

```javascript
const db = FerrumDB.open("secure.db", {
  encryptionKey: "my_super_secret_key_32_bytes_!!?"  // exactly 32 characters
});

db.set("secret", { classified: true });
```

---

## Ferrum Studio (Web Dashboard)

Launch the built-in web dashboard directly from your app:

```javascript
const db = FerrumDB.open("myapp.db");
db.startStudio(7474); // http://localhost:7474
```

Browse keys, inspect values, set/delete entries, and view real-time metrics — no extra tools needed.

---

## API Reference

### FerrumDB

| Method | Description |
|---|---|
| `FerrumDB.open(path, options?)` | Opens or creates a database. Options: `{ encryptionKey: string }` |
| `get(key)` | Retrieves a value by key. Returns `null` if not found. |
| `set(key, value)` | Stores a JSON-serializable value. |
| `setEx(key, value, ttlSeconds)` | Stores a value with TTL (auto-expires). |
| `delete(key)` | Deletes a key. Returns `true` if existed. |
| `keys()` | Returns all keys. |
| `count()` | Total entries. |
| `createIndex(field)` | Creates a secondary index on a JSON field. |
| `find(field, value)` | Queries keys by indexed field value. |
| `commit(tx)` | Commits an atomic transaction. |
| `startStudio(port?)` | Launches Ferrum Studio dashboard. Default port: `7474`. |

### Transaction

| Method | Description |
|---|---|
| `new Transaction()` | Creates a new transaction. |
| `set(key, value)` | Stages a SET operation. |
| `setEx(key, value, ttlSeconds)` | Stages a SET with TTL. |
| `delete(key)` | Stages a DELETE operation. |

---

## License

MIT — [Muhammad Usman](https://github.com/MuhammadUsmanGM)
