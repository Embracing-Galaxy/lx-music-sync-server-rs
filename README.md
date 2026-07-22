# LX Music Sync Server (rs)

[![Rust](https://img.shields.io/badge/Rust-1.85%2B-blue?logo=rust)](https://www.rust-lang.org)
[![Edition](https://img.shields.io/badge/edition-2024-blue)](https://doc.rust-lang.org/edition-guide/)

Server-side sync engine for the **LX Music** ecosystem. Real-time playlist and dislike-list synchronization across
devices via WebSocket. Single binary, no external database — JSON-file persistence.

Pre-compiled binaries for `x86_64-unknown-linux-gnu` and `x86_64-unknown-linux-musl` are available on the [Releases](https://github.com/Embracing-Galaxy/lx-music-sync-server-rs/releases) page.

## Quick Start

### Prerequisites

- **Rust ≥1.85** (edition 2024)
- **OpenSSL** (linked via `openssl` crate)

### Build & Run

```bash
cargo build --release
cargo run
```

Server starts on `127.0.0.1:9527` by default.

## Configuration

Auto-creates `config.json` on first run. Add users under `user_configs`:

```json
{
  "server_name": "My Sync Server",
  "enable_proxy": false,
  "user_configs": {
    "alice": {
      "password": "secret123",
      "max_snapshot_count": 10,
      "add_music_location": "top"
    }
  }
}
```

| Field          | Type   | Description                          |
|----------------|--------|--------------------------------------|
| `server_name`  | string | Display name shown to clients        |
| `enable_proxy` | bool   | Use `X-Real-IP` header for client IP |
| `user_configs` | map    | Map of username → user config        |

Per-user fields:

| Field                | Type   | Default | Description             |
|----------------------|--------|---------|-------------------------|
| `password`           | string | —       | Auth password           |
| `max_snapshot_count` | number | —       | Max snapshots to retain |
| `add_music_location` | enum   | `"top"` | `"top"` / `"bottom"`    |

### Runtime Data Layout

```
working-dir/
├── config.json              # Server config
├── server_info.json         # Server identity
└── users/
    └── <username>/
        ├── devices.json     # Registered devices
        ├── list/            # Playlist snapshots
        └── dislike/         # Dislike list snapshots
```
