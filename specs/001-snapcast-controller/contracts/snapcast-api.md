# Snapcast JSON-RPC API Contract

**Date**: 2025-12-12
**Feature**: 001-snapcast-controller
**API Version**: Snapcast JSON-RPC 2.0

## Overview

This document defines the subset of the Snapcast JSON-RPC API that the controller application will use. The Snapcast server exposes a JSON-RPC 2.0 interface over TCP (default port 1705).

## Connection

**Protocol**: JSON-RPC 2.0 over TCP
**Default Port**: 1705
**Transport**: Persistent TCP connection with newline-delimited JSON messages

## Methods Used

### 1. Server.GetStatus

**Purpose**: Retrieve complete server state including all clients, groups, and streams

**Request**:
```json
{
  "id": 1,
  "jsonrpc": "2.0",
  "method": "Server.GetStatus"
}
```

**Response**:
```json
{
  "id": 1,
  "jsonrpc": "2.0",
  "result": {
    "server": {
      "host": {
        "name": "snapserver-hostname"
      },
      "snapserver": {
        "version": "0.27.0"
      }
    },
    "streams": [
      {
        "id": "stream-1",
        "status": "playing",
        "uri": {
          "raw": "spotify:///librespot",
          "scheme": "spotify"
        },
        "meta": {
          "STREAM": "Spotify"
        }
      },
      {
        "id": "stream-2",
        "status": "idle",
        "uri": {
          "raw": "pipe:///tmp/snapfifo",
          "scheme": "pipe"
        },
        "meta": {
          "STREAM": "Radio"
        }
      }
    ],
    "groups": [
      {
        "id": "group-1",
        "name": "Living Room Group",
        "stream_id": "stream-1",
        "muted": false,
        "clients": [
          {
            "id": "living-room",
            "host": {
              "name": "living-room-pi"
            },
            "config": {
              "name": "Living Room",
              "volume": {
                "muted": false,
                "percent": 75
              }
            },
            "connected": true,
            "lastSeen": {
              "sec": 1670000000,
              "usec": 123456
            }
          }
        ]
      }
    ]
  }
}
```

**Key Fields**:
- `result.streams[]`: Array of audio streams
  - `id`: Unique stream identifier
  - `status`: "playing", "idle", "unknown"
  - `meta.STREAM`: Human-readable stream name
- `result.groups[].clients[]`: Array of clients (rooms)
  - `id`: Client identifier (matches our config `room_client_id`)
  - `config.name`: Display name
  - `config.volume.percent`: Volume 0-100
  - `config.volume.muted`: Mute status
  - `connected`: Online status
- `result.groups[].stream_id`: Stream assigned to group containing this client

**Usage**: Called on initial connection and periodically to sync state

---

### 2. Client.SetVolume

**Purpose**: Set volume level for a specific client

**Request**:
```json
{
  "id": 2,
  "jsonrpc": "2.0",
  "method": "Client.SetVolume",
  "params": {
    "id": "living-room",
    "volume": {
      "percent": 80,
      "muted": false
    }
  }
}
```

**Parameters**:
- `id`: Client ID (string)
- `volume.percent`: Volume level 0-100 (integer)
- `volume.muted`: Mute status (boolean)

**Response**:
```json
{
  "id": 2,
  "jsonrpc": "2.0",
  "result": {
    "volume": {
      "percent": 80,
      "muted": false
    }
  }
}
```

**Error Response**:
```json
{
  "id": 2,
  "jsonrpc": "2.0",
  "error": {
    "code": -32602,
    "message": "Invalid client ID"
  }
}
```

**Usage**: Called when user rotates volume knob

---

### 3. Group.SetStream

**Purpose**: Assign a group to a different audio stream

**Request**:
```json
{
  "id": 3,
  "jsonrpc": "2.0",
  "method": "Group.SetStream",
  "params": {
    "id": "group-1",
    "stream_id": "stream-2"
  }
}
```

**Parameters**:
- `id`: Group ID (string) - Found from Server.GetStatus by locating group containing our client
- `stream_id`: Target stream ID (string)

**Response**:
```json
{
  "id": 3,
  "jsonrpc": "2.0",
  "result": {
    "stream_id": "stream-2"
  }
}
```

**Usage**: Called when user selects a different stream via buttons

**Note**: Clients are organized into groups. To change a client's stream, we must:
1. Find which group contains our client (from Server.GetStatus)
2. Call Group.SetStream with that group's ID
3. This affects all clients in the group

---

## Notifications (Server-Initiated)

The Snapcast server sends notifications when state changes. These are JSON-RPC notifications (no `id` field).

### 1. Client.OnVolumeChanged

**Notification**:
```json
{
  "jsonrpc": "2.0",
  "method": "Client.OnVolumeChanged",
  "params": {
    "id": "living-room",
    "volume": {
      "percent": 85,
      "muted": false
    }
  }
}
```

**Purpose**: Notifies when client volume or mute status changes (from any source, including our controller)

**Usage**: Update `RoomState.volume` and `RoomState.muted`, refresh hardware screens

---

### 2. Client.OnConnect

**Notification**:
```json
{
  "jsonrpc": "2.0",
  "method": "Client.OnConnect",
  "params": {
    "id": "living-room",
    "client": {
      "id": "living-room",
      "host": {
        "name": "living-room-pi"
      },
      "config": {
        "name": "Living Room",
        "volume": {
          "muted": false,
          "percent": 75
        }
      },
      "connected": true
    }
  }
}
```

**Purpose**: Notifies when a client connects to the server

**Usage**: Set `RoomState.connected = true` if client matches our configured room

---

### 3. Client.OnDisconnect

**Notification**:
```json
{
  "jsonrpc": "2.0",
  "method": "Client.OnDisconnect",
  "params": {
    "id": "living-room",
    "client": {
      "id": "living-room",
      "connected": false
    }
  }
}
```

**Purpose**: Notifies when a client disconnects from the server

**Usage**: Set `RoomState.connected = false`, display "Disconnected" on hardware screens

---

### 4. Stream.OnUpdate

**Notification**:
```json
{
  "jsonrpc": "2.0",
  "method": "Stream.OnUpdate",
  "params": {
    "id": "stream-1",
    "stream": {
      "id": "stream-1",
      "status": "playing",
      "meta": {
        "STREAM": "Spotify",
        "ARTIST": "Artist Name",
        "TITLE": "Song Title",
        "ALBUM": "Album Name"
      }
    }
  }
}
```

**Purpose**: Notifies when stream status or metadata changes

**Usage**: Update `AudioStream.status` and `AudioStream.metadata` in streams list

---

### 5. Group.OnStreamChanged

**Notification**:
```json
{
  "jsonrpc": "2.0",
  "method": "Group.OnStreamChanged",
  "params": {
    "id": "group-1",
    "stream_id": "stream-2"
  }
}
```

**Purpose**: Notifies when a group is assigned to a different stream

**Usage**: If group contains our client, update `RoomState.stream_id` and refresh screens

---

## Error Codes

Standard JSON-RPC 2.0 error codes:

| Code | Meaning | Common Cause |
|------|---------|--------------|
| -32700 | Parse error | Invalid JSON |
| -32600 | Invalid request | Malformed JSON-RPC |
| -32601 | Method not found | Unknown method name |
| -32602 | Invalid params | Wrong parameter types or missing required params |
| -32603 | Internal error | Server-side failure |

**Error Handling Strategy**:
- Parse errors (-32700): Log and reconnect
- Invalid method (-32601): Bug in our code, log and skip
- Invalid params (-32602): Display error on screen ("Invalid room" or "Invalid stream")
- Internal error (-32603): Retry once, then display error

---

## Connection Management

### Initial Connection Flow
1. Open TCP connection to `server_address:server_port`
2. Send `Server.GetStatus` request
3. Wait for response and parse initial state
4. Subscribe to notifications (automatic - server sends all notifications to connected clients)

### Reconnection Strategy
- Detect disconnection via TCP socket error or read timeout
- Close socket
- Wait 1 second (first retry)
- Double wait time on each failure (exponential backoff): 1s, 2s, 4s, 8s, max 60s
- Display "Reconnecting..." on hardware screens
- On successful reconnect, send `Server.GetStatus` to resync state

### Keepalive
- No explicit keepalive required
- Notifications from server serve as keepalive
- If no message received for 60 seconds, consider connection stale and reconnect

---

## Implementation Notes

### Request ID Management
- Use monotonically increasing integers for request IDs
- Track pending requests to match responses
- Timeout requests after 5 seconds

### Message Framing
- Messages are newline-delimited (`\n`)
- Each line is a complete JSON object
- Use buffered reader to accumulate until newline

### Concurrency
- Separate tokio task for reading from socket (notifications)
- Separate tokio task for writing to socket (requests)
- Use mpsc channel to queue outgoing requests

### State Synchronization
- On connect: `Server.GetStatus` is source of truth
- During operation: Notifications update state incrementally
- On notification loss (gap detected): Re-request `Server.GetStatus`

---

## Example Session

```
→ Client connects to server
← Server accepts connection

→ {"id":1,"jsonrpc":"2.0","method":"Server.GetStatus"}
← {"id":1,"jsonrpc":"2.0","result":{...full status...}}

→ {"id":2,"jsonrpc":"2.0","method":"Client.SetVolume","params":{"id":"living-room","volume":{"percent":80,"muted":false}}}
← {"id":2,"jsonrpc":"2.0","result":{"volume":{"percent":80,"muted":false}}}
← {"jsonrpc":"2.0","method":"Client.OnVolumeChanged","params":{"id":"living-room","volume":{"percent":80,"muted":false}}}

→ {"id":3,"jsonrpc":"2.0","method":"Group.SetStream","params":{"id":"group-1","stream_id":"stream-2"}}
← {"id":3,"jsonrpc":"2.0","result":{"stream_id":"stream-2"}}
← {"jsonrpc":"2.0","method":"Group.OnStreamChanged","params":{"id":"group-1","stream_id":"stream-2"}}
```

---

## Security Considerations

**Authentication**: Snapcast API v0.27+ may support authentication. For MVP:
- Assume no authentication required (local network)
- Add auth support if needed during implementation

**Network Security**:
- Unencrypted TCP (no TLS)
- Intended for trusted local network only
- User responsible for network security (firewall, VPN, etc.)

**Input Validation**:
- Validate all JSON responses match expected schema
- Sanitize client/stream IDs before display on hardware screens
- Clamp volume values to 0-100 range
