# Research: Home Assistant Amplifier Control

**Feature**: 002-homeassistant-amplifier-control
**Date**: 2025-12-22
**Status**: Complete

## Overview

This document captures technical research and decisions made during the planning phase for integrating Home Assistant amplifier control into the Ajazz hardware controller.

## Home Assistant Integration Approach

### Decision: MQTT API

**Chosen**: Use Home Assistant MQTT API for bidirectional communication

**Rationale**:
- User has Home Assistant with MQTT integration already configured
- MQTT provides publish/subscribe pattern ideal for real-time state updates
- Mature Rust MQTT clients available (rumqttc, paho-mqtt)
- Lightweight protocol suitable for local network communication
- Native support in Home Assistant for entity state publishing
- Simpler than WebSocket API - no authentication complexity, no long-polling

**Alternatives Considered**:
- **Home Assistant WebSocket API**: More complex authentication, requires managing WebSocket connections, would need custom REST client for commands. Rejected due to added complexity.
- **Home Assistant REST API**: Requires polling for state updates, higher latency, more network overhead. Rejected due to performance requirements (<2s state updates).
- **Custom Rust Home Assistant client**: Would need to implement both WebSocket and REST. Rejected per Simplicity First principle.

### MQTT Client Library

**Decision**: Use `rumqttc` crate

**Rationale**:
- Pure Rust implementation (no C dependencies)
- Async/await support with tokio integration
- Active maintenance (last update within 6 months)
- MQTTv5 and MQTTv3.1.1 support
- Good documentation and examples
- Lightweight and performant

**Alternatives Considered**:
- **paho-mqtt-rust**: Bindings to Eclipse Paho C library, adds build complexity with C dependencies
- **mqtt-async**: Less mature, fewer features
- **Custom MQTT implementation**: Massive scope increase, violates Simplicity First

## Home Assistant MQTT Topics

### State Updates (Subscribe)

Home Assistant publishes entity states to MQTT topics following this pattern:

```
homeassistant/{component}/{node_id}/{object_id}/state
```

**For amplifier control**:
- Power switch: `homeassistant/switch/amplifier/power/state` → payload: `ON` | `OFF`
- Input source: `homeassistant/select/amplifier/source/state` → payload: source name string

**Alternative discovery pattern**:
```
homeassistant/{component}/{node_id}/{object_id}/config
```
Contains JSON with entity metadata, state topic, command topic

### Command Publishing (Publish)

To control entities, publish to command topics:

```
homeassistant/{component}/{node_id}/{object_id}/set
```

**For amplifier control**:
- Toggle power: Publish `ON` or `OFF` to `homeassistant/switch/amplifier/power/set`
- Change source: Publish source name to `homeassistant/select/amplifier/source/set`

## Entity Configuration

### Expected Home Assistant Entity IDs

**Power Switch**:
- Entity ID: `switch.amplifier_power` (or user-configured)
- Entity Type: `switch`
- States: `on`, `off`
- Attributes: `friendly_name`, `device_class`

**Input Source Selector**:
- Entity ID: `input_select.amplifier_source` (or user-configured)
- Entity Type: `input_select` or `select`
- Current state: Selected source name
- Attributes: `options` (list of available sources), `friendly_name`

### Configuration Requirements

User must configure in `config.toml`:
```toml
[homeassistant]
broker_address = "192.168.1.100"
broker_port = 1883
# Optional authentication
username = "controller"
password = "secret"

[homeassistant.amplifier]
power_entity = "switch.amplifier_power"
source_entity = "input_select.amplifier_source"
```

## Integration Patterns

### Concurrent Operation with Snapcast

**Challenge**: Application already has Snapcast client running, need to add Home Assistant MQTT client without blocking or race conditions.

**Solution**: Spawn separate tokio task for MQTT client, use channels for event communication.

```rust
// In main.rs event loop structure:
let (ha_event_tx, ha_event_rx) = mpsc::channel()
let (ha_command_tx, ha_command_rx) = mpsc::channel()

// Spawn Home Assistant MQTT task
tokio::spawn(async move {
    homeassistant::client::mqtt_loop(config, ha_event_tx, ha_command_rx).await
});

// Main loop receives events from both Snapcast and Home Assistant
loop {
    select! {
        Some(hw_event) = hardware_event_rx.recv() => { /* handle */ }
        Some(snap_event) = snapcast_event_rx.recv() => { /* handle */ }
        Some(ha_event) = ha_event_rx.recv() => { /* handle Home Assistant */ }
    }
}
```

### State Management

**Extend ApplicationState** in `src/controller/state.rs`:
```rust
pub struct ApplicationState {
    // Existing fields...
    pub config: ConnectionSettings,
    pub room: Option<RoomState>,
    pub current_page: PageView,

    // New fields for Home Assistant
    pub homeassistant_connected: bool,
    pub amplifier_power: Option<bool>,  // None = unknown
    pub amplifier_source: Option<String>,
    pub available_sources: Vec<String>,
}
```

**Extend PageView enum**:
```rust
pub enum PageView {
    Status,              // Snapcast status (existing)
    StreamSelection,     // Snapcast streams (existing)
    Settings,            // Snapcast settings (existing)
    AmplifierControl,    // NEW - amplifier power/source
}
```

### Error Handling

**Connection Failures**:
- Detect MQTT broker unreachable on connect attempt
- Display "Home Assistant Disconnected" on amplifier control page
- Queue commands for retry when connection restored (up to 10 commands)
- Log connection errors to stderr

**Command Failures**:
- Timeout if no state update received within 5 seconds after command
- Display error message on hardware screen: "Command Failed"
- Allow retry by user pressing button again

**State Inconsistency**:
- If entity state not received within 10 seconds of subscription, mark as "Unknown"
- Display "Unknown" state on hardware controller
- Continue attempting to receive updates

## Display Layout Design

### Amplifier Control Page

**6-button layout** (matching existing Snapcast pages):

```
┌─────────┬─────────┬─────────┐
│ Button 0│ Button 1│ Button 2│
│  POWER  │ SOURCE  │ VOLUME  │  (if amplifier volume separate from Snapcast)
│  [ON]   │ [CD]    │  N/A    │
└─────────┴─────────┴─────────┘
┌─────────┬─────────┬─────────┐
│ Button 3│ Button 4│ Button 5│
│  HA     │ (empty) │ (empty) │
│Connected│         │         │
└─────────┴─────────┴─────────┘
```

**Button 0 - Power**:
- Top label: "POWER"
- Center: "ON" or "OFF" (large text)
- Visual: Green background for ON, red for OFF

**Button 1 - Source**:
- Top label: "SOURCE"
- Center: Current source name (truncated if >10 chars)
- Press to cycle through sources or open source selection

**Button 3 - Home Assistant Status**:
- Top label: "HA"
- Center: "Connected" or "Disconnected"
- Visual: Green for connected, red for disconnected

### Source Selection Sub-page

When button 1 pressed, show source selection similar to Snapcast stream selection:

```
Each button shows one source option:
┌─────────┬─────────┬─────────┐
│   CD    │  AUX    │  TV     │
│   [>]   │         │         │  (> indicates currently selected)
└─────────┴─────────┴─────────┘
┌─────────┬─────────┬─────────┐
│ OPTICAL │ (empty) │  BACK   │
│         │         │         │
└─────────┴─────────┴─────────┘
```

Press source button to select, automatically returns to main amplifier control page.

## Performance Considerations

### Network Latency

- MQTT broker on local network: typically 5-20ms
- Entity state update propagation through Home Assistant: 50-100ms
- Display update on hardware: 100-200ms
- **Total expected latency**: 155-320ms (well under 2s requirement)

### Message Throughput

- State updates: ~1-5 per second during active use
- Command publishes: ~1 per second max (human interaction speed)
- MQTT QoS 1 (at least once delivery) adequate for this use case
- No performance concerns with expected message rates

### Memory Usage

- rumqttc client: ~100KB
- Entity state storage: ~1KB (2 entities with metadata)
- Configuration: ~500 bytes
- **Total incremental memory**: ~100KB (negligible for Linux host)

## Dependencies to Add

```toml
[dependencies]
# MQTT client
rumqttc = "0.24"  # Async MQTT client

# Existing dependencies remain unchanged
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
toml = "0.8"
# ... others
```

## Security Considerations

### Authentication

- MQTT broker may require username/password
- Store credentials in config file with appropriate file permissions (0600)
- No need for TLS in local network scenario (can add later if needed)

### Input Validation

- Validate entity IDs match expected pattern before subscribing
- Sanitize source names before display (limit length, filter special chars)
- Validate state values (ON/OFF for switch, source name in available list)

## Testing Strategy

Per constitution, tests are optional. Manual validation:

1. **Connection Test**: Start application, verify "HA Connected" on display
2. **Power Control**: Toggle power via button, verify amplifier responds and display updates
3. **Source Selection**: Cycle through sources, verify amplifier switches input
4. **State Sync**: Change power/source in Home Assistant UI, verify controller display updates
5. **Reconnection**: Disconnect MQTT broker, verify error display, reconnect and verify recovery
6. **Concurrent Use**: Operate Snapcast and amplifier controls simultaneously, verify no interference

## Implementation Notes

### Simplicity First Alignment

- No retry/backoff library - simple loop with tokio::time::sleep
- No connection pooling - single MQTT client connection
- No caching layer - direct subscribe/publish
- No event sourcing - immediate state updates
- Configuration via existing TOML pattern (extend ConnectionSettings)

### Code Reuse

- Copy Snapcast client structure for Home Assistant client module
- Reuse hardware display manager for new page layouts
- Extend ApplicationState rather than create separate state machine
- Use existing event channel pattern for MQTT events

### Future Extensibility (not implemented now)

- Additional amplifier controls (bass/treble/balance) → add more entities
- Multiple amplifiers → extend config with array, add amplifier selector page
- Zone control → add zone entities, extend display
- TLS support → add rustls dependency, extend config

These are noted but NOT implemented per Simplicity First principle.

## Open Questions Resolved

1. **Which Home Assistant API?** → MQTT (user specified)
2. **MQTT client library?** → rumqttc (mature, pure Rust, tokio support)
3. **Entity naming?** → Configurable in TOML, defaults documented
4. **State synchronization?** → Subscribe to state topics, immediate updates
5. **Command pattern?** → Publish to command topics with QoS 1
6. **Integration with Snapcast?** → Parallel tokio tasks, shared event loop
7. **Display layout?** → New PageView::AmplifierControl, 6-button layout
8. **Source selection UX?** → Sub-page similar to Snapcast stream selection

All technical clarifications resolved. Ready for Phase 1 (data model and contracts).
