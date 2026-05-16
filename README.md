# LX Music Sync Server (rs)

[![Rust](https://img.shields.io/badge/Rust-1.85%2B-blue?logo=rust)](https://www.rust-lang.org)
[![Edition](https://img.shields.io/badge/edition-2024-blue)](https://doc.rust-lang.org/edition-guide/)

Server-side sync engine for the **LX Music** ecosystem. Real-time playlist and dislike-list synchronization across
devices via WebSocket. Single binary, no external database — JSON-file persistence.

---

## Features

- **Real-time sync** — playlist and dislike list changes propagate to all connected devices instantly via WebSocket
  broadcast
- **Conflict-free merge** — snapshot-based 3-way merge algorithm handles concurrent edits across devices
- **Dual auth** — password-based enrollment for first-time devices, key-based auth for subsequent connections
- **No external DB** — everything persisted as JSON files (config, snapshots, device registry)
- **Single binary** — built with Rust + tokio, zero runtime dependencies
- **Automatic compression** — large payloads are transparently gzipped
- **Heartbeat** — 30s keepalive with timeout detection
- **IP rate-limiting** — blocks after repeated failed auth attempts

---

## Quick Start

### Prerequisites

- **Rust ≥1.85** (edition 2024)
- **OpenSSL** (linked via `openssl` crate)

### Build & Run

```bash
# Build
cargo build --release

# Run
cargo run
```

The server starts on `127.0.0.1:9527` by default.

### Useful Commands

```bash
cargo check          # Type-check (fast)
cargo clippy         # Lint
cargo fmt            # Format
cargo test           # Run tests
```

---

## Configuration

### `config.json`

Auto-created on first run. Add users under `user_configs`:

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

#### Per-User Fields

| Field                | Type   | Default | Description                                     |
|----------------------|--------|---------|-------------------------------------------------|
| `password`           | string | —       | Auth password                                   |
| `max_snapshot_count` | number | —       | Max snapshots to retain                         |
| `add_music_location` | enum   | `"top"` | Where new music is placed: `"top"` / `"bottom"` |

### `server_info.json`

Auto-generated with a random server ID on first run. Do not modify.

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

---

## API

### `GET /hello`

Health check. Returns a greeting string.

### `GET /id`

Returns the server ID to help clients identify this server.

### `GET /ah` — Auth & handshake

Authenticates a device and returns credentials for WebSocket connections.

**Headers:**

| Header | Required | Description                    |
|--------|----------|--------------------------------|
| `m`    | Yes      | Encrypted auth message         |
| `i`    | No       | Client ID (for key-based auth) |

Two flows:

- **Code-based** (no `i` header): Authenticate with password, receive device credentials (`clientId` + `key`).
- **Key-based** (with `i` header): Authenticate with existing device key.

### `GET /socket` — WebSocket

Upgrades to a WebSocket connection. Requires `?i=<clientId>` (obtained from `/ah`).

---

## WebSocket Protocol

All messages are JSON-encoded text frames. The protocol uses a request/response pattern with automatic correlation.

### Initial Sync

On connection, the server synchronizes the client's playlist and dislike data to the latest server state, merging
concurrently with any other connected devices.

### Real-time Propagation

When any device modifies data, the change is broadcast to all other connected devices automatically.

### Heartbeat

The server sends a keepalive every 30 seconds and closes connections that fail to respond.

---

## Auth Overview

### First-time device

The client sends an encrypted auth request with the user's password. The server validates and returns device credentials
that are stored locally for future connections.

### Returning device

The client sends an encrypted auth request using the previously received device key. No password needed.

