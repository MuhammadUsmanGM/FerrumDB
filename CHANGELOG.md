# Changelog

All notable changes to FerrumDB will be documented in this file.

---

## [0.1.3] — 2026-04-04

### Added
- **TTL support in bindings** — `setEx()` / `set_ex()` for both Node.js and Python (+ transactions)
- **Encryption in bindings** — `FerrumDB.open(path, { encryptionKey })` (Node.js) / `FerrumDB.open(path, encryption_key=)` (Python)
- **`startStudio()` / `start_studio()`** — Launch Ferrum Studio web dashboard directly from Node.js or Python, no Cargo needed
- Updated TypeScript declarations (`index.d.ts`) with all new methods

---

## [0.1.2] — 2026-04-04

### Added
- **`ferrumdb-cli`** — Standalone CLI tool (`cargo install ferrumdb-cli`)
  - `ferrumdb web <path>` — Launch Ferrum Studio dashboard for any `.db` file
  - `ferrumdb info <path>` — Show key count and file size
  - `ferrumdb compact <path>` — Remove deleted/expired entries, reclaim disk space
- Ferrum Studio section added to Python and Node.js READMEs
- Python README created for PyPI

### Changed
- Ferrum Studio dashboard HTML is now public API (`ferrumdb::studio::DASHBOARD_HTML`), enabling reuse by the CLI

---

## [0.1.1] — 2026-04-03

### Added
- **Node.js bindings** via NAPI-RS (`npm install ferrumdb`)
  - Full API: `open`, `get`, `set`, `delete`, `keys`, `count`, `createIndex`, `find`, `commit`
  - `Transaction` class for atomic batch operations
- **Python bindings** via PyO3 (`pip install ferrumdb`)
  - Full API: `open`, `get`, `set`, `delete`, `keys`, `count`, `create_index`, `find`, `commit`
  - `Transaction` class for atomic batch operations
- **Example projects** in `examples/`:
  - `rust-example` — Task Manager (CRUD, indexes, transactions)
  - `python-example` — Contact Book (CRUD, indexes)
  - `node-example` — Note Taker (CRUD, indexes, transactions)
- **Ferrum Studio** — Built-in web dashboard at `localhost:7474` with live metrics

### Changed
- **Bitcask offset index** — Index now stores `(offset, size)` instead of full values in memory, reducing RAM usage significantly
- **Bincode serialization** — On-disk format switched from JSON to bincode for smaller files and faster I/O
- **Lazy expiry** — Expired keys are evicted on read with double-checked locking instead of upfront scans
- **Crash-safe recovery** — Corrupt trailing records are skipped instead of returning hard errors
- **Windows-safe rename** — `compact()` now deletes target file before rename for Windows compatibility
- **Compaction race fix** — Write locks held for entire compaction duration to prevent data loss
- **Single-write atomicity** — Length prefix + data combined into one `write_all` call

---

## [0.1.0] — Initial Release

### Added
- Append-only log (Bitcask-inspired) storage engine
- In-memory `HashMap` index rebuilt on startup
- AES-256-GCM encryption at rest (per-block, random nonce)
- Secondary indexing on top-level JSON fields
- Atomic transactions (all-or-nothing batch writes)
- Configurable fsync policy (`Always` / `Periodic(ms)` / `Never`)
- Log compaction with atomic file swap
- CLI REPL with autocomplete and syntax highlighting
- Lock-free atomic metrics (ops/sec, uptime, GET/SET/DELETE counts)
- Criterion benchmarks
