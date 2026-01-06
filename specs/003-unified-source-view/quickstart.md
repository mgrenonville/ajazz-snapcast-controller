# Developer Quickstart: Unified Source View

**Feature**: 003-unified-source-view
**Date**: 2026-01-05
**Audience**: Developers working on or extending the unified source abstraction

---

## Overview

The Unified Source View feature presents both Snapcast audio streams and amplifier input sources as a single, cohesive list that users can navigate and select from. This guide explains how the abstraction works, how to extend it, and how to troubleshoot common issues.

---

## Architecture at a Glance

```
┌─────────────────────────────────────────────────────────────────┐
│                     User Interface Layer                         │
│                  (hardware/display.rs)                           │
│              UnifiedSourceSelection Page View                    │
└──────────────────────┬──────────────────────────────────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────────────────────────┐
│                 Application State Layer                          │
│                  (controller/state.rs)                           │
│            ApplicationState::unified_sources                     │
└──────────────────────┬──────────────────────────────────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────────────────────────┐
│               Unified Source Abstraction                         │
│               (models/unified_source.rs)                         │
│   ┌──────────────────────┬──────────────────────────┐          │
│   │ UnifiedSource enum   │ UnifiedSourceCollection  │          │
│   │ (data model)         │ (state management)       │          │
│   └──────────────────────┴──────────────────────────┘          │
│   ┌──────────────────────────────────────────────────┐          │
│   │ SourceActivationContext                          │          │
│   │ (multi-step orchestration)                       │          │
│   └──────────────────────────────────────────────────┘          │
└───────────────┬─────────────────────────┬────────────────────────┘
                │                         │
                ▼                         ▼
┌───────────────────────────┐  ┌─────────────────────────────────┐
│   Snapcast Integration    │  │  Home Assistant Integration     │
│  (snapcast/client.rs)     │  │  (homeassistant/client.rs)      │
│                           │  │                                 │
│  - Stream listing         │  │  - Amplifier source selection   │
│  - Stream selection       │  │  - IR command transmission      │
│  - Real-time events       │  │  - MQTT communication           │
└───────────────────────────┘  └─────────────────────────────────┘
```

---

## Key Concepts

### 1. UnifiedSource Enum

The core abstraction that represents "something the user can listen to":

```rust
pub enum UnifiedSource {
    SnapcastStream { stream_id: String, name: String },
    AmplifierInput { source: AmplifierSource, name: String },
}
```

**Design rationale**: Enum variants with associated data provide type safety and exhaustive matching. The compiler ensures we handle both Snapcast and amplifier sources everywhere.

### 2. Snapcast-as-Amplifier-Input Mapping

**Key insight**: Snapcast is physically connected to one of the amplifier's input jacks. When a user selects a Snapcast stream, we need to:
1. Switch the amplifier to the "Snapcast input" (e.g., "Spotify" jack)
2. Tell Snapcast which stream to play

This is configured in `config.toml`:

```toml
[homeassistant.amplifier]
snapcast_amplifier_source = "Spotify"  # Which amp input has Snapcast connected
```

### 3. Multi-Step Activation

Selecting a Snapcast stream requires **two operations**:
- Amplifier IR command: Switch to Snapcast input
- Snapcast JSON-RPC: Select the stream

Selecting a non-Snapcast amplifier input requires **one operation**:
- Amplifier IR command: Switch to that input

This logic is encapsulated in `SourceActivationContext::activate()`.

---

## Common Tasks

### Task 1: Add Support for a New Amplifier Source

**Scenario**: Your amplifier has a 6th input (e.g., "Bluetooth") and you want it in the unified view.

**Steps**:

1. **Add the new variant to `AmplifierSource` enum** (`src/homeassistant/types.rs`):

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum AmplifierSource {
    Phono,
    CD,
    Spotify,
    Source4,
    Source5,
    Bluetooth,  // NEW
}
```

2. **Add display name** in `display_name()` method:

```rust
impl AmplifierSource {
    pub fn display_name(&self) -> &'static str {
        match self {
            // ... existing cases
            AmplifierSource::Bluetooth => "Bluetooth",
        }
    }
}
```

3. **Add IR code** (`src/homeassistant/ir_codes.rs`):

```rust
pub mod sources {
    // ... existing codes
    pub const BLUETOOTH: u16 = 0xC06;  // Your IR code here
}

impl AmplifierSource {
    pub fn ir_code(&self) -> u16 {
        // ... existing cases
        AmplifierSource::Bluetooth => sources::BLUETOOTH,
    }
}
```

4. **Update `AmplifierSource::all()`**:

```rust
pub fn all() -> [AmplifierSource; 6] {  // Update array size
    [
        AmplifierSource::Phono,
        AmplifierSource::CD,
        AmplifierSource::Spotify,
        AmplifierSource::Source4,
        AmplifierSource::Source5,
        AmplifierSource::Bluetooth,  // NEW
    ]
}
```

5. **Rebuild** and the new source will automatically appear in the unified view!

**Why it works**: `UnifiedSourceCollection::new()` iterates over `AmplifierSource::all()` and creates `UnifiedSource::AmplifierInput` entries for each (except the one configured for Snapcast).

---

### Task 2: Change Which Amplifier Input Snapcast Uses

**Scenario**: You moved your Snapcast device from the "Spotify" jack to the "CD" jack.

**Steps**:

1. **Update `config.toml`**:

```toml
[homeassistant.amplifier]
power_entity = "switch.amplifier_power"
snapcast_amplifier_source = "CD"  # Changed from "Spotify"
```

2. **Restart the application**. That's it!

**What happens**:
- The "CD" amplifier input will no longer appear as a separate entry in the unified source list
- Selecting any Snapcast stream will switch the amplifier to "CD" instead of "Spotify"
- All Snapcast streams will still be visible and selectable

---

### Task 3: Debug Source Activation Issues

**Scenario**: User selects a Snapcast stream but nothing happens.

**Debug checklist**:

1. **Check logs** for activation errors:

```bash
RUST_LOG=debug cargo run
# Look for lines like:
# ERROR unified_source: Failed to activate source: ...
```

2. **Verify Home Assistant connectivity**:

```rust
// In your debug session
println!("HA connected: {}", state.homeassistant_connected);
println!("Amplifier state: {:?}", state.amplifier);
```

3. **Verify Snapcast connectivity**:

```rust
println!("Snapcast connected: {}", state.server_connected);
println!("Room state: {:?}", state.room);
```

4. **Check configuration**:

```rust
println!("Snapcast amplifier source: {:?}",
    state.config.homeassistant.as_ref()
        .map(|ha| ha.amplifier.snapcast_amplifier_source));
```

5. **Test IR commands manually**:

Use the amplifier control page to verify IR blaster is working:
- Try switching sources manually
- Check MQTT broker logs for published messages

6. **Test Snapcast commands manually**:

Use snapcast CLI or web interface to verify stream selection works:

```bash
snapcast-client -h <server-ip> -c "SetStream <client-id> <stream-id>"
```

---

### Task 4: Customize Source Display Names

**Scenario**: You want Snapcast stream "stream_spotify" to display as "Spotify Connect" instead of the server-provided name.

**Current limitation**: Display names come directly from Snapcast server stream metadata. Customization would require:

1. **Add name mapping to configuration**:

```toml
[homeassistant.amplifier]
snapcast_amplifier_source = "Spotify"

[[homeassistant.amplifier.source_name_overrides]]
stream_id = "stream_spotify"
display_name = "Spotify Connect"
```

2. **Update `UnifiedSourceCollection::new()` to apply overrides**:

```rust
// In update_snapcast_streams()
for stream in snapcast_streams {
    let display_name = self.name_overrides
        .get(&stream.stream_id)
        .cloned()
        .unwrap_or(stream.name);

    sources.push(UnifiedSource::SnapcastStream {
        stream_id: stream.stream_id,
        name: display_name,
    });
}
```

**Note**: This is a future enhancement not included in MVP.

---

## Code Navigation Guide

### Where to Find Things

| Component | File | What's There |
|-----------|------|--------------|
| **Core data model** | `src/models/unified_source.rs` | `UnifiedSource` enum, `UnifiedSourceCollection`, `SourceActivationContext` |
| **Application state** | `src/controller/state.rs` | `ApplicationState`, unified source state management methods |
| **Display rendering** | `src/hardware/display.rs` | Page rendering logic (add `UnifiedSourceSelection` page here) |
| **Button mapping** | `src/controller/mapping.rs` | Hardware button to action mapping |
| **Configuration** | `src/config/settings.rs` | Config structure with `snapcast_amplifier_source` field |
| **Snapcast integration** | `src/snapcast/client.rs` | Snapcast JSON-RPC client, event handling |
| **Home Assistant integration** | `src/homeassistant/client.rs` | MQTT client, IR command publishing |
| **IR codes** | `src/homeassistant/ir_codes.rs` | RC5 command codes for amplifier |

### Key Methods to Understand

1. **`UnifiedSourceCollection::new()`**: Builds initial source list from Snapcast streams + amplifier inputs
2. **`UnifiedSourceCollection::update_snapcast_streams()`**: Rebuilds collection when stream list changes
3. **`SourceActivationContext::activate()`**: Orchestrates multi-step source activation
4. **`ApplicationState::select_unified_source()`**: High-level method called when user confirms selection
5. **`ApplicationState::handle_snapcast_event()`**: Processes Snapcast events to keep unified view in sync

---

## State Synchronization Flow

### When Snapcast Streams Change

```
Snapcast Server          snapcast/client.rs        ApplicationState       UnifiedSourceCollection
      │                         │                          │                         │
      │  StreamUpdate event     │                          │                         │
      ├────────────────────────>│                          │                         │
      │                         │  SnapcastEvent           │                         │
      │                         ├─────────────────────────>│                         │
      │                         │                          │  update_snapcast_...()  │
      │                         │                          ├────────────────────────>│
      │                         │                          │                         │
      │                         │                          │  Rebuild sources list   │
      │                         │                          │<────────────────────────│
      │                         │                          │                         │
      │                         │  Trigger display refresh │                         │
      │                         │<─────────────────────────│                         │
```

### When User Selects a Source

```
Hardware Button       controller/mapping.rs    ApplicationState     SourceActivationContext
      │                       │                       │                       │
      │  Button press         │                       │                       │
      ├──────────────────────>│                       │                       │
      │                       │  select_unified_...() │                       │
      │                       ├──────────────────────>│                       │
      │                       │                       │  Create context       │
      │                       │                       ├──────────────────────>│
      │                       │                       │                       │
      │                       │                       │  activate()           │
      │                       │                       │──────────────────────>│
      │                       │                       │                       │
      │                       │                       │  → HA: Select source  │
      │                       │                       │  → Snapcast: Set strm │
      │                       │                       │<──────────────────────│
      │                       │                       │                       │
      │                       │  Update active source │                       │
      │                       │  Persist state        │                       │
      │                       │  Trigger refresh      │                       │
```

---

## Testing Your Changes

### Manual Testing Checklist

After making changes to unified source logic:

- [ ] **View unified source list**: Verify all expected sources appear
- [ ] **Navigate with knob**: Verify scrolling works smoothly
- [ ] **Select Snapcast stream**: Verify amplifier switches to Snapcast input AND stream changes
- [ ] **Select amplifier input**: Verify amplifier switches to correct input
- [ ] **Restart application**: Verify last active source is restored from state file
- [ ] **Disconnect Snapcast server**: Verify graceful handling (streams disappear from list)
- [ ] **Reconnect Snapcast server**: Verify streams reappear in list
- [ ] **Change config `snapcast_amplifier_source`**: Verify correct amplifier input used for Snapcast

### Unit Testing

Run existing tests:

```bash
cargo test unified_source
```

Add new tests when adding features:

```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_my_new_feature() {
        // Test implementation
    }
}
```

---

## Performance Optimization Tips

### Minimize Collection Rebuilds

`UnifiedSourceCollection::update_snapcast_streams()` is relatively expensive (rebuilds entire list). Only call it when:
- Snapcast streams actually change (not on every `StreamUpdate` event)
- Application starts up
- Snapcast server reconnects

**Good pattern**:

```rust
// Only rebuild if stream list actually changed
if snapcast_streams != self.cached_snapcast_streams {
    self.unified_sources.update_snapcast_streams(snapcast_streams.clone());
    self.cached_snapcast_streams = snapcast_streams;
}
```

### Lazy State Persistence

Don't persist state on every source activation - only when it changes:

```rust
// In ApplicationState::select_unified_source()
let old_active = self.unified_sources.active_source().cloned();

// ... activate source ...

// Only persist if active source actually changed
if old_active.as_ref() != Some(&new_source) {
    self.save_unified_source_state()?;
}
```

---

## Configuration Reference

### Complete TOML Example

```toml
[server]
address = "192.168.1.100"
port = 1705

[room]
client_id = "living-room-controller"

[homeassistant]
broker_address = "192.168.1.100"
broker_port = 1883
ir_blaster_topic = "tasmota_17DD9F/cmnd/irsend"

[homeassistant.amplifier]
power_entity = "switch.amplifier_power"
snapcast_amplifier_source = "Spotify"  # Which amplifier input Snapcast uses

# Optional future enhancements:
# [features]
# unified_source_view = true  # Enable unified view (default: true)
```

---

## Troubleshooting Common Issues

### Issue: "Snapcast streams don't appear in unified view"

**Possible causes**:
1. Snapcast server not connected → Check `state.server_connected`
2. No streams configured on server → Check Snapcast server config
3. Streams filtered out incorrectly → Check `update_snapcast_streams()` logic

**Fix**: Verify Snapcast connectivity and stream list with `snapcast-control` CLI.

---

### Issue: "Selecting Snapcast stream switches amplifier but not stream"

**Possible causes**:
1. Snapcast client ID mismatch → Check `state.room.client_id` matches your client
2. Stream ID invalid → Verify stream exists on server
3. Snapcast command failed → Check logs for JSON-RPC errors

**Fix**: Verify `client_id` in config matches Snapcast server client ID.

---

### Issue: "Amplifier doesn't switch when selecting source"

**Possible causes**:
1. MQTT broker disconnected → Check `state.homeassistant_connected`
2. IR blaster offline → Check IR blaster device connectivity
3. Wrong IR code → Verify IR codes in `ir_codes.rs` match your amplifier

**Fix**: Test IR commands manually using MQTT explorer or `mosquitto_pub`.

---

### Issue: "State not persisted across restarts"

**Possible causes**:
1. Config directory not writable → Check permissions on `~/.config/snapcast-controller/`
2. JSON serialization error → Check logs for serde errors
3. State file corrupted → Delete `unified_source_state.json` and retry

**Fix**: Check file permissions and logs for write errors.

---

## Next Steps

After understanding the unified source view:

1. **Read the spec** (`spec.md`) for user stories and requirements
2. **Review the plan** (`plan.md`) for architectural decisions
3. **Check the data model** (`data-model.md`) for detailed struct definitions
4. **Read contracts** (`contracts/unified-source-interface.md`) for API expectations
5. **Run the code** and experiment with source selection

For implementation tasks, see `tasks.md` (generated by `/speckit.tasks`).

---

## Questions?

For questions or issues with the unified source view:
1. Check this quickstart guide
2. Review the research document (`research.md`) for design rationale
3. Examine existing code in `src/models/unified_source.rs`
4. Check application logs with `RUST_LOG=debug`

Happy coding!
