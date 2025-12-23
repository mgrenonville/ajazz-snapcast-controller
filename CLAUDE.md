# ajazz-snapcast-controller Development Guidelines

Auto-generated from all feature plans. Last updated: 2025-12-12

## Active Technologies
- File-based configuration (TOML files in ~/.config or similar) (002-homeassistant-amplifier-control)

- Rust (stable channel, edition 2021 or later) (001-snapcast-controller)

## Project Structure

```text
src/
tests/
```

## Commands

cargo test [ONLY COMMANDS FOR ACTIVE TECHNOLOGIES][ONLY COMMANDS FOR ACTIVE TECHNOLOGIES] cargo clippy

## Code Style

Rust (stable channel, edition 2021 or later): Follow standard conventions

## Recent Changes
- 002-homeassistant-amplifier-control: Added Rust (stable channel, edition 2021 or later)

- 001-snapcast-controller: Added Rust (stable channel, edition 2021 or later)

<!-- MANUAL ADDITIONS START -->

## Home Assistant Integration Patterns

### IR Blaster Integration
- **Architecture**: One-way IR command transmission via MQTT to Tasmota IR Blaster device
- **Protocol**: RC5 IR protocol with 12-bit command codes
- **Topic**: Configurable MQTT topic (`ir_blaster_topic` in config.toml)
- **Use Cases**: Source selection and volume control for amplifier
- **Pattern**: Send-only commands with no state feedback from IR-controlled devices

### State Management Patterns

#### Hybrid State Approach
The application uses different state management strategies based on device capabilities:

1. **Bidirectional State (Home Assistant Entity)**
   - **Example**: Amplifier power state
   - **Pattern**: Subscribe to MQTT entity state topic, receive updates
   - **Storage**: State maintained by Home Assistant
   - **Sync**: Real-time updates via MQTT notifications

2. **Local State with Persistence**
   - **Example**: Selected amplifier source
   - **Pattern**: Track state locally, persist to file
   - **Storage**: JSON file in `~/.config/snapcast-controller/amplifier_state.json`
   - **Rationale**: IR-controlled devices provide no state feedback
   - **Persistence**: Survives application restarts

3. **Remote State (Snapcast)**
   - **Example**: Room volume, stream selection
   - **Pattern**: Subscribe to Snapcast server notifications
   - **Storage**: State maintained by Snapcast server
   - **Sync**: Real-time updates via JSON-RPC notifications

### MQTT Communication Patterns

#### Publishing IR Commands
```rust
// Build IR command structure
IrCommand {
    protocol: "RC5",
    bits: 12,
    data: 0xC01,  // RC5 code for source/function
    repeat: 1,
}
// Publish to configurable IR Blaster topic as JSON
```

#### Subscribing to Entity State
```rust
// Subscribe to Home Assistant entity state changes
// Topic: homeassistant/switch/amplifier_power/state
// Payload: "ON" or "OFF"
```

### Configuration Pattern
```toml
[homeassistant]
broker_address = "192.168.1.100"
broker_port = 1883
ir_blaster_topic = "tasmota_17DD9F/cmnd/irsend"  # Configurable per device

[homeassistant.amplifier]
power_entity = "switch.amplifier_power"  # HA entity for power state
```

### Design Principles
1. **Separation of Concerns**: Power state (HA entity) vs. source selection (IR commands)
2. **No Assumptions**: IR commands are fire-and-forget, no state assumptions
3. **User Experience**: Local state provides immediate UI feedback
4. **Resilience**: State persistence survives disconnections and restarts

<!-- MANUAL ADDITIONS END -->
