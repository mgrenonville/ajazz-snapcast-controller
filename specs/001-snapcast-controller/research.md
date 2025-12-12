# Technical Research: Snapcast Controller Application

**Date**: 2025-12-12
**Feature**: 001-snapcast-controller

## Overview

This document resolves technical uncertainties identified in the Technical Context section of plan.md, focusing on dependency selection and architectural patterns for the Snapcast controller application.

## Research Topics

### 1. Async Runtime Selection

**Decision**: **Tokio**

**Rationale**:
- Most mature and widely adopted async runtime in Rust ecosystem
- Likely already used by `snapcast_control` crate (JSON-RPC typically async)
- Excellent ecosystem support and documentation
- Performance is proven for I/O-bound workloads (network + USB events)
- Simplicity principle: Use what the ecosystem standardizes on

**Alternatives Considered**:
- **async-std**: More "std-like" API but smaller ecosystem, less adoption in networked applications
- **smol**: Lightweight but requires more manual setup, less ecosystem support for JSON-RPC clients

**Implementation Notes**:
- Use `tokio::main` macro for async entry point
- Use tokio channels for USB → network event bridging
- Single tokio runtime handles both USB HID events and Snapcast network I/O

---

### 2. Configuration Management

**Decision**: **TOML + serde**

**Rationale**:
- TOML is human-friendly for manual editing (users will configure server address, room ID)
- `serde` + `toml` crate is standard Rust pattern (minimal dependencies)
- No need for complex configuration library - simple file read/write suffices
- Aligns with Simplicity First principle: avoid framework overhead

**Alternatives Considered**:
- **config-rs**: Overkill for simple key-value config (server address, port, room ID)
- **figment**: More features than needed, adds unnecessary complexity

**Configuration Schema**:
```toml
[server]
address = "192.168.1.100"
port = 1705

[room]
client_id = "living-room"

[hardware]
# Future: USB device identifiers if multiple controllers
```

**Implementation Notes**:
- Config file location: `~/.config/snapcast-controller/config.toml` (Linux)
- Use `serde::Deserialize` for type-safe loading
- Write example config on first run if missing

---

### 3. Error Handling Approach

**Decision**: **anyhow** for application errors, **thiserror** for library boundaries

**Rationale**:
- **anyhow**: Perfect for application-level error propagation with context chaining
  - Simplifies main.rs and controller logic error handling
  - Good error messages for debugging (critical during prototyping)
- **thiserror**: Use only for defining custom error types at module boundaries
  - Hardware module errors, Snapcast module errors
  - Provides structured error types for pattern matching where needed

**Alternatives Considered**:
- **Custom Result types**: Too much boilerplate for rapid prototyping
- **Pure thiserror**: More verbose than needed for application-level code
- **Pure anyhow**: Less structured at module boundaries where specific error handling matters

**Error Handling Strategy**:
1. Hardware module: Define `HardwareError` enum with thiserror (DeviceNotFound, ReadError, WriteError)
2. Snapcast module: Define `SnapcastError` enum with thiserror (ConnectionFailed, InvalidRoom, CommandError)
3. Main application: Use anyhow::Result everywhere, add `.context()` for debugging
4. Display errors on hardware controller screens (user-facing error messages)

---

### 4. Hardware Controller Integration

**Research**: `ajazz_sdk` crate capabilities

**Findings**:
- Assumed API based on typical HID SDKs (exact details TBD during implementation):
  - Device enumeration and connection
  - Event listening for knobs (rotation events with delta values)
  - Event listening for buttons (press/release events)
  - Screen update API (likely takes buffer or image per button screen)
  - Page button handling for UI state management

**Implementation Approach**:
- Spawn dedicated task for HID event loop (tokio::task)
- Send hardware events to main controller via tokio::mpsc channel
- Hardware event types: `KnobRotated(id, delta)`, `ButtonPressed(id)`, `ButtonReleased(id)`, `PageButton(id)`
- Screen updates queued and rendered in batches to avoid USB bandwidth saturation

---

### 5. Snapcast API Integration

**Research**: `snapcast_control` crate and Snapcast JSON-RPC protocol

**Findings** (based on Snapcast documentation):
- JSON-RPC 2.0 over TCP (default port 1705)
- Key methods needed:
  - `Server.GetStatus` - Get all clients, groups, and streams
  - `Client.SetVolume` - Set volume for specific client
  - `Client.SetMute` - Mute/unmute client
  - `Group.SetStream` - Assign group to stream
- Server sends notifications for state changes (enables real-time status updates)

**Implementation Approach**:
- Maintain persistent connection to Snapcast server
- Subscribe to server notifications for room state changes
- Command pattern: Each control action (volume, mute, stream) → JSON-RPC method call
- State caching: Keep local copy of room state, update from notifications
- Reconnection logic: Detect disconnection, attempt reconnect with exponential backoff

---

### 6. Application Architecture Pattern

**Decision**: **Event-driven architecture with tokio**

**Rationale**:
- Two event sources: Hardware (USB) and Network (Snapcast)
- Events must be processed concurrently without blocking
- Simple state machine: Connected/Disconnected states for both hardware and server

**Architecture**:
```
┌──────────────┐         ┌───────────────────┐         ┌─────────────────┐
│  USB HID     │ events  │   Controller      │ commands│  Snapcast       │
│  (ajazz_sdk) │────────>│   State Machine   │────────>│  Server         │
│              │         │                   │         │  (JSON-RPC)     │
│              │<────────│   - Room State    │<────────│                 │
└──────────────┘ screens └───────────────────┘ notifs  └─────────────────┘
       ▲                                                        │
       │                                                        │
       └────────────────── Update Screens ─────────────────────┘
```

**Event Loop**:
1. Main tokio runtime spawns two tasks: USB listener, Snapcast listener
2. Both tasks send events to central controller via mpsc channels
3. Controller task processes events, updates state, sends commands
4. Controller updates hardware screens when state changes

---

## Technology Stack Summary

| Component | Technology | Justification |
|-----------|------------|---------------|
| Language | Rust (stable, edition 2021) | Per user requirement |
| Async Runtime | Tokio | Industry standard, best ecosystem support |
| Hardware SDK | ajazz_sdk | Per user requirement |
| Snapcast Client | snapcast_control | Per user requirement |
| Config Format | TOML + serde | Simple, human-friendly |
| Error Handling | anyhow + thiserror | Pragmatic balance of simplicity and structure |
| State Management | In-memory struct | No persistence needed beyond config |

---

## Open Questions for Implementation Phase

1. **ajazz_sdk API details**: Exact method signatures, event types, screen buffer format (resolve during P1/P2 user stories)
2. **Screen rendering**: Text-only vs graphics? Resolution per button screen? (discover during hardware testing)
3. **Knob mapping**: Which knob controls volume? Are others for future features? (define in quickstart.md)
4. **Button assignments**: How to map 6 buttons to mute, stream selection, other actions? (define in quickstart.md)
5. **Page button behavior**: Switch between stream list pages, settings, status views? (design in Phase 1)

These questions will be resolved iteratively during implementation per the "Speed Over Perfection" constitution principle.
