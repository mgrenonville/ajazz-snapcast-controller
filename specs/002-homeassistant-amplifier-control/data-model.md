# Data Model: Home Assistant Amplifier Control

**Feature**: 002-homeassistant-amplifier-control
**Date**: 2025-12-22
**Status**: Complete

## Overview

This document defines the data entities, their relationships, and state transitions for the Home Assistant amplifier control feature. The model extends the existing Snapcast controller data structures.

## Entity Definitions

### AmplifierState

Represents the current state of the amplifier as known by the controller.

**Fields**:
- `power_on: Option<bool>` - Amplifier power state (None = unknown, Some(true) = on, Some(false) = off)
- `current_source: Option<String>` - Currently selected input source (None = unknown)
- `available_sources: Vec<String>` - List of available input sources
- `last_updated: std::time::Instant` - Timestamp of last state update
- `availability: EntityAvailability` - Whether the entity is reachable

**Validation Rules**:
- `available_sources` must not be empty when `current_source` is Some
- `current_source` must be in `available_sources` list when both are known
- `last_updated` timestamp must not be in the future

**State Transitions**:
```
Unknown (None) -> Known (Some) when MQTT state received
Known -> Unknown when connection lost for >10 seconds
On -> Off when power toggle command sent and confirmed
Off -> On when power toggle command sent and confirmed
SourceA -> SourceB when source selection command sent and confirmed
```

**Relationships**:
- Contained within `ApplicationState`
- Updated by `HomeAssistantEvent::EntityStateChanged`
- Displayed on `PageView::AmplifierControl` page

---

### HomeAssistantConnection

Represents the connection state to the Home Assistant MQTT broker.

**Fields**:
- `broker_address: String` - MQTT broker hostname or IP
- `broker_port: u16` - MQTT broker port (typically 1883)
- `username: Option<String>` - MQTT authentication username
- `password: Option<String>` - MQTT authentication password (stored securely)
- `client_id: String` - MQTT client identifier
- `connected: bool` - Current connection status
- `last_connect_attempt: Option<std::time::Instant>` - Timestamp of last connection attempt
- `reconnect_delay: std::time::Duration` - Current reconnection backoff delay

**Validation Rules**:
- `broker_address` must not be empty
- `broker_port` must be in range 1-65535
- `client_id` must be unique and not empty
- `reconnect_delay` must be between 1 second and 60 seconds

**State Transitions**:
```
Disconnected -> Connecting when connection attempt initiated
Connecting -> Connected when MQTT CONNACK received
Connecting -> Disconnected when connection fails
Connected -> Disconnected when connection lost or intentionally closed
Disconnected -> Connecting (with backoff) on automatic reconnection
```

**Relationships**:
- Configured from `HomeAssistantConfig` in TOML file
- Managed by `homeassistant::client::MqttClient`
- Status displayed on `PageView::AmplifierControl` (button 3)

---

### HomeAssistantConfig

Configuration for Home Assistant integration, loaded from TOML file.

**Fields**:
- `broker_address: String` - MQTT broker address
- `broker_port: u16` - MQTT broker port
- `username: Option<String>` - Optional authentication username
- `password: Option<String>` - Optional authentication password
- `amplifier: AmplifierEntityConfig` - Amplifier entity configuration

**Validation Rules**:
- All rules from `HomeAssistantConnection` for broker fields
- Must pass `validate()` method before use

**Source**: Loaded from `config.toml`:
```toml
[homeassistant]
broker_address = "192.168.1.100"
broker_port = 1883
username = "controller"  # optional
password = "secret"      # optional

[homeassistant.amplifier]
power_entity = "switch.amplifier_power"
source_entity = "input_select.amplifier_source"
```

**Relationships**:
- Part of `ConnectionSettings` (extends existing structure)
- Used to initialize `HomeAssistantConnection`
- Persisted between application restarts

---

### AmplifierEntityConfig

Configuration for Home Assistant entity IDs used for amplifier control.

**Fields**:
- `power_entity: String` - Entity ID for power switch (e.g., "switch.amplifier_power")
- `source_entity: String` - Entity ID for input selector (e.g., "input_select.amplifier_source")

**Validation Rules**:
- Entity IDs must match pattern: `{domain}.{object_id}`
- Power entity domain must be "switch"
- Source entity domain must be "input_select" or "select"
- Object IDs must be non-empty and contain only alphanumeric, underscore

**MQTT Topic Mapping**:
- Power state: `homeassistant/switch/{object_id}/state`
- Power command: `homeassistant/switch/{object_id}/set`
- Source state: `homeassistant/input_select/{object_id}/state`
- Source command: `homeassistant/input_select/{object_id}/set`

**Relationships**:
- Part of `HomeAssistantConfig`
- Used to construct MQTT topic subscriptions

---

### EntityAvailability

Enumeration representing entity reachability status.

**Values**:
- `Available` - Entity is reachable and responding
- `Unavailable` - Entity explicitly marked unavailable by Home Assistant
- `Unknown` - Availability status unknown (no recent updates)

**Transitions**:
```
Unknown -> Available when first state update received
Available -> Unavailable when availability:offline message received
Unavailable -> Available when availability:online message received
Any -> Unknown after 30 seconds without state update
```

**Display Mapping**:
- `Available` → Display entity state normally
- `Unavailable` → Display "Offline" on hardware screen
- `Unknown` → Display "Unknown" or "?" on hardware screen

---

### HomeAssistantEvent

Events emitted by the Home Assistant client to the application event loop.

**Variants**:

1. **`BrokerConnected`**
   - Emitted when MQTT connection established
   - No additional data
   - Triggers: Update UI connection status to "Connected"

2. **`BrokerDisconnected`**
   - Emitted when MQTT connection lost
   - No additional data
   - Triggers: Update UI connection status to "Disconnected", mark entities Unknown

3. **`EntityStateChanged { entity_id: String, state: String }`**
   - Emitted when entity state update received via MQTT
   - `entity_id`: Full entity ID (e.g., "switch.amplifier_power")
   - `state`: New state value (e.g., "on", "off", source name)
   - Triggers: Update `AmplifierState`, refresh display if on amplifier control page

4. **`EntityAvailabilityChanged { entity_id: String, available: bool }`**
   - Emitted when entity availability changes
   - Triggers: Update `AmplifierState.availability`

5. **`CommandAcknowledged { entity_id: String }`**
   - Emitted when command successfully published to MQTT broker
   - Provides immediate feedback before state update received
   - Triggers: Visual feedback on hardware (loading indicator)

6. **`CommandFailed { entity_id: String, error: String }`**
   - Emitted when command fails to publish or times out
   - Triggers: Display error message on hardware screen

**Relationships**:
- Produced by `homeassistant::client::MqttClient`
- Consumed by main event loop in `main.rs`
- Handled by `ApplicationState::handle_homeassistant_event()`

---

### HomeAssistantCommand

Commands sent from the application to the Home Assistant client.

**Variants**:

1. **`TogglePower`**
   - Toggle amplifier power (on→off or off→on)
   - Client determines target state based on current `AmplifierState.power_on`
   - Publishes "ON" or "OFF" to power command topic

2. **`SetPower { on: bool }`**
   - Set amplifier power to specific state
   - `on`: true = power on, false = power off
   - Publishes "ON" or "OFF" to power command topic

3. **`SelectSource { source_name: String }`**
   - Change amplifier input source
   - `source_name`: Must be in `AmplifierState.available_sources`
   - Publishes source name to source command topic

4. **`Disconnect`**
   - Gracefully disconnect from MQTT broker
   - Used during application shutdown

**Validation**:
- Commands validated before execution
- `TogglePower`: Requires `AmplifierState.power_on` is Some (known state)
- `SelectSource`: Requires `source_name` in `available_sources` list
- Failed validation → emit `CommandFailed` event

**Relationships**:
- Sent via `mpsc::UnboundedSender<HomeAssistantCommand>`
- Received by `homeassistant::client::MqttClient`
- Triggered by hardware button press events

---

## Extended Entities

### ApplicationState (Extended)

Existing `ApplicationState` struct extended with Home Assistant fields:

**New Fields**:
```rust
pub struct ApplicationState {
    // Existing Snapcast fields...
    pub config: ConnectionSettings,
    pub room: Option<RoomState>,
    pub streams: Vec<AudioStream>,
    pub hardware_connected: bool,
    pub server_connected: bool,
    pub current_page: PageView,

    // NEW: Home Assistant fields
    pub homeassistant_connected: bool,
    pub amplifier: Option<AmplifierState>,
}
```

**New Methods**:
- `handle_homeassistant_event(&mut self, event: HomeAssistantEvent) -> bool` - Returns true if screen refresh needed
- `get_amplifier_state(&self) -> Option<&AmplifierState>` - Get current amplifier state
- `can_toggle_power(&self) -> bool` - Check if power toggle is allowed
- `can_select_source(&self, source: &str) -> bool` - Check if source selection is allowed

---

### PageView (Extended)

Existing `PageView` enum extended with new page:

```rust
pub enum PageView {
    Status,              // Existing - Snapcast room status
    StreamSelection,     // Existing - Snapcast stream selection
    Settings,            // Existing - Snapcast settings
    AmplifierControl,    // NEW - Amplifier power and source control
}
```

**Page Navigation**:
- Page button 0: Status (Snapcast)
- Page button 1: StreamSelection (Snapcast)
- Page button 2: AmplifierControl (NEW)
- Pressing current page button cycles to next page

---

### ConnectionSettings (Extended)

Existing configuration structure extended:

```rust
pub struct ConnectionSettings {
    pub server: ServerConfig,      // Existing - Snapcast server
    pub room: RoomConfig,           // Existing - Snapcast room
    pub homeassistant: Option<HomeAssistantConfig>,  // NEW - Optional HA config
}
```

**Backward Compatibility**:
- `homeassistant` field is `Option` - existing configs without HA still work
- If `homeassistant` is `None`, amplifier control page shows "Not Configured"

---

## Data Flow

### Startup Sequence

1. Load `config.toml` → parse into `ConnectionSettings`
2. Validate `homeassistant` config if present
3. Initialize `ApplicationState` with `amplifier: None`, `homeassistant_connected: false`
4. Spawn MQTT client task if `homeassistant` config present
5. MQTT client connects → emits `BrokerConnected` event
6. Subscribe to entity state topics
7. Receive initial state updates → populate `AmplifierState`
8. Update display if user on `AmplifierControl` page

### State Update Flow

```
Home Assistant entity changes
    ↓
MQTT broker publishes to state topic
    ↓
rumqttc client receives message
    ↓
Parse MQTT payload → extract state value
    ↓
Emit HomeAssistantEvent::EntityStateChanged
    ↓
Main event loop receives event
    ↓
ApplicationState.handle_homeassistant_event()
    ↓
Update AmplifierState fields
    ↓
Return true (needs refresh)
    ↓
Build AmplifierControlPageLayout
    ↓
Send HardwareCommand::UpdateAmplifierPage
    ↓
Display manager renders to hardware screens
```

### Command Flow

```
User presses button on hardware
    ↓
HardwareEvent::ButtonPressed { button_id }
    ↓
Main event loop receives event
    ↓
Determine current page (AmplifierControl)
    ↓
Map button to action (button 0 = toggle power)
    ↓
Validate command (check state is known)
    ↓
Send HomeAssistantCommand::TogglePower
    ↓
MQTT client receives command
    ↓
Determine target state (on→off or off→on)
    ↓
Publish to MQTT command topic
    ↓
Emit CommandAcknowledged event
    ↓
Display "Sending..." feedback
    ↓
Wait for state update (handled by State Update Flow)
```

### Error Handling Flow

```
Command validation fails
    ↓
Return error to main loop
    ↓
Send HardwareCommand::ShowError("Cannot toggle power: state unknown")
    ↓
Display error on hardware screen for 3 seconds
    ↓
Return to previous display
```

---

## Persistence

### Configuration Persistence

**File**: `~/.config/snapcast-controller/config.toml` (or similar)

**Format**:
```toml
[server]
address = "192.168.1.50"
port = 1705

[room]
client_id = "living_room"

[homeassistant]
broker_address = "192.168.1.100"
broker_port = 1883
username = "controller"
password = "secret"

[homeassistant.amplifier]
power_entity = "switch.amplifier_power"
source_entity = "input_select.amplifier_source"
```

**Persistence Guarantee**:
- Configuration loaded once at startup
- No runtime writes to config file
- Changes require application restart (acceptable per Simplicity First)

### State Persistence

**No state persistence required**:
- Amplifier state is ephemeral (queried from Home Assistant on startup)
- MQTT client receives current state on connection
- Connection state is transient (reconnects automatically)

**Rationale**: Persisting stale state could cause inconsistencies. Querying fresh state on startup is simpler and more reliable.

---

## Memory Layout Estimates

```
AmplifierState: ~200 bytes
  - Option<bool>: 2 bytes
  - Option<String>: ~40 bytes (typical source name)
  - Vec<String>: ~200 bytes (5 sources × 40 bytes)
  - Instant: 16 bytes
  - EntityAvailability: 1 byte

HomeAssistantConnection: ~150 bytes
  - Strings: ~100 bytes (broker address, username, client_id)
  - Port, bool, timestamps: ~50 bytes

HomeAssistantConfig: ~200 bytes
  - Similar to HomeAssistantConnection

Total incremental memory for HA feature: ~550 bytes
```

**Memory Impact**: Negligible (<1KB). No performance concerns.

---

## Concurrency Model

### Thread Safety

- `ApplicationState`: Owned by main thread, not shared
- MQTT client: Runs in separate tokio task
- Communication via `mpsc` channels (Send + Sync)
- No shared mutable state between tasks

### Channel Sizing

- `ha_event_tx/rx`: Unbounded (events processed quickly, no backpressure needed)
- `ha_command_tx/rx`: Unbounded (command rate limited by human interaction)

### Locking Strategy

- No mutexes needed (each task owns its state)
- Channel sends are lock-free
- MQTT client state not shared with main thread

---

## Summary

This data model extends the existing Snapcast controller architecture with Home Assistant integration while maintaining simplicity:

- **4 new core entities**: AmplifierState, HomeAssistantConnection, HomeAssistantConfig, AmplifierEntityConfig
- **2 enums**: EntityAvailability, HomeAssistantEvent/Command
- **2 extended entities**: ApplicationState, PageView
- **No new database or persistence layer** - configuration via existing TOML pattern
- **Event-driven architecture** - fits existing Snapcast pattern
- **~550 bytes memory overhead** - negligible impact

All entities follow existing patterns from Snapcast module for consistency and simplicity.
