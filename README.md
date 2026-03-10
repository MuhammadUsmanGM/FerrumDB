# FerrumDB

A terminal-based key-value database written in Rust, featuring persistent storage, safe concurrent access, and structured logging.

```
  | __| ___  _ _  _ _  _  _ _ _| _ \| _ )
  | _| / -_)| '_|| '_|| || | '  \  _/| _ \
  |_|  \___||_|  |_|   \_,_|_|_|_|  |___/
```

## Features

- **In-memory KV store** backed by `HashMap` with JSON file persistence
- **Concurrent access** via `tokio::sync::RwLock` (multiple readers, exclusive writer)
- **Interactive REPL** with command history (arrow keys) powered by `rustyline`
- **Structured logging** via `tracing` with configurable log levels
- **Operation metrics** tracking GETs, SETs, DELETEs, errors, and uptime
- **Graceful shutdown** on `EXIT`, `Ctrl+C`, or `Ctrl+D`

## Getting Started

### Prerequisites

- [Rust](https://rustup.rs/) (1.70+ recommended)

### Build & Run

```bash
cargo run
```

Release build for better performance:

```bash
cargo run --release
```

### Run Tests

```bash
cargo test
```

Run stress tests (ignored by default):

```bash
cargo test --release -- --ignored
```

## Commands

| Command | Description |
|---------|-------------|
| `SET <key> <value>` | Store a key-value pair (values can contain spaces) |
| `GET <key>` | Retrieve the value for a key |
| `DELETE <key>` | Remove a key-value pair |
| `KEYS` | List all stored keys |
| `COUNT` | Show total number of entries |
| `HELP` | Show help and session metrics |
| `EXIT` | Save and quit |

## Example Session

```
ferrumdb> SET name usman
OK
ferrumdb> SET language rust
OK
ferrumdb> GET name
usman
ferrumdb> KEYS
  language
  name
(2 keys)
ferrumdb> DELETE name
OK (deleted)
ferrumdb> GET name
(nil)
ferrumdb> EXIT
Goodbye!
```

## Architecture

```
src/
├── main.rs       # Entry point, async runtime, REPL loop
├── storage.rs    # Storage engine (HashMap + RwLock + JSON persistence)
├── cli.rs        # Command parser (SET, GET, DELETE, KEYS, COUNT, HELP, EXIT)
├── error.rs      # Unified error types
└── metrics.rs    # Atomic operation counters and uptime tracking
```

### Concurrency Model

- `tokio::sync::RwLock` for thread-safe read/write access to the KV store
- `AtomicU64` for lock-free metrics counters
- `Arc` for shared ownership across async tasks

### Persistence

Data is stored in `ferrumdb.json` in the working directory. The file is written on every SET/DELETE operation and loaded automatically on startup.

## Configuration

Set the `RUST_LOG` environment variable for verbose logging:

```bash
RUST_LOG=ferrumdb=debug cargo run
```

## License

MIT
