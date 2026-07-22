# API

## `GET /hello`
Health check. Returns a greeting string.

## `GET /id`
Returns the server ID to help clients identify this server.

## `GET /ah` — Auth & handshake
Authenticates a device and returns credentials for WebSocket connections.

**Headers:**

| Header | Required | Description                    |
|--------|----------|--------------------------------|
| `m`    | Yes      | Encrypted auth message         |
| `i`    | No       | Client ID (for key-based auth) |

Two flows:

- **Code-based** (no `i` header): Authenticate with password, receive device credentials (`clientId` + `key`).
- **Key-based** (with `i` header): Authenticate with existing device key.

## `GET /socket` — WebSocket
Upgrades to a WebSocket connection. Requires `?i=<clientId>` (obtained from `/ah`).

---

# WebSocket Protocol

All messages are JSON-encoded text frames. Request/response pattern with automatic correlation.

### Initial Sync
On connection, the server synchronizes the client's playlist and dislike data to the latest server state, merging concurrently with other connected devices.

### Real-time Propagation
Changes from any device are broadcast to all other connected devices automatically.

### Heartbeat
Server sends a keepalive every 30s; closes connections that fail to respond.
