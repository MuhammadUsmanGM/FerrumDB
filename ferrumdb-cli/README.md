# FerrumDB CLI

Standalone command-line tool for FerrumDB. Launch the Ferrum Studio web dashboard, inspect databases, and run compaction — works with any `.db` file regardless of whether you use Rust, Python, or Node.js.

---

## Installation

```bash
cargo install ferrumdb-cli
```

---

## Commands

### `ferrumdb web` — Launch Ferrum Studio

```bash
ferrumdb web myapp.db
ferrumdb web myapp.db --port 8080
```

Opens the Ferrum Studio web dashboard at `http://localhost:7474` (default). Browse keys, inspect values, set/delete entries, and view real-time metrics.

### `ferrumdb info` — Database Info

```bash
ferrumdb info myapp.db
```

Shows key count and file size.

### `ferrumdb compact` — Compact Database

```bash
ferrumdb compact myapp.db
```

Removes deleted and expired entries, reclaiming disk space.

---

## License

MIT — [Muhammad Usman](https://github.com/MuhammadUsmanGM)
