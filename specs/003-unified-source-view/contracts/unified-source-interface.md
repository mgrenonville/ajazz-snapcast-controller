# API Contract: Unified Source Interface

**Feature**: 003-unified-source-view
**Date**: 2026-01-05
**Purpose**: Define internal API contracts for unified source abstraction

---

## Overview

This document specifies the internal Rust trait interfaces and communication contracts for the unified source abstraction layer. These contracts define how the unified source module integrates with existing Snapcast and Home Assistant subsystems.

---

## Module Interfaces

### 1. UnifiedSourceManager Trait

**Purpose**: Core interface for managing unified source collection and activation

**Location**: `src/models/unified_source.rs`

```rust
use async_trait::async_trait;
use anyhow::Result;

#[async_trait]
pub trait UnifiedSourceManager {
    /// Get all available unified sources
    fn all_sources(&self) -> &[UnifiedSource];

    /// Get currently active source
    fn active_source(&self) -> Option<&UnifiedSource>;

    /// Update available Snapcast streams
    fn update_snapcast_streams(&mut self, streams: Vec<AudioStream>);

    /// Set active source and persist state
    async fn set_active_source(&mut self, source: UnifiedSource) -> Result<()>;

    /// Activate a unified source (handles multi-step operations)
    async fn activate_source(
        &mut self,
        source: &UnifiedSource,
        snapcast_client: &SnapcastClient,
        ha_client: &HomeAssistantClient,
    ) -> Result<ActivationResult>;

    /// Load persisted state from disk
    fn load_persisted_state(&mut self) -> Result<()>;

    /// Save current state to disk
    fn save_state(&self) -> Result<()>;
}
```

**Contract Guarantees**:
- `all_sources()` returns sorted list (Snapcast streams first, then amplifier inputs)
- `update_snapcast_streams()` preserves non-Snapcast sources in collection
- `activate_source()` completes within 2 seconds or returns error
- `save_state()` is atomic (temp file + rename pattern)

---

### 2. Integration with SnapcastClient

**Existing Interface**: `src/snapcast/client.rs`

**Contract**: Unified source module calls existing Snapcast client methods without modification

```rust
// Existing SnapcastClient interface (NO CHANGES)
impl SnapcastClient {
    /// Send command to Snapcast server
    pub async fn send_command(&self, command: SnapcastCommand) -> Result<()>;

    /// Subscribe to server events
    pub fn subscribe_events(&self) -> Receiver<SnapcastEvent>;
}
```

**Usage by Unified Source**:

```rust
// In SourceActivationContext::activate()
snapcast_client.send_command(SnapcastCommand::SetStream {
    client_id: self.snapcast_client_id.clone(),
    stream_id: stream_id.clone(),
}).await?;
```

**Event Handling**:

```rust
// In ApplicationState::handle_snapcast_event()
match event {
    SnapcastEvent::ServerReconnected { streams, .. } => {
        // Update unified source collection with new stream list
        self.unified_sources.update_snapcast_streams(streams);
    }
    SnapcastEvent::StreamUpdate { stream, .. } => {
        // Rebuild unified sources if stream list changed
        // (handled by periodic refresh or explicit rebuild trigger)
    }
    // ... other events
}
```

---

### 3. Integration with HomeAssistantClient

**Existing Interface**: `src/homeassistant/client.rs`

**Contract**: Unified source module calls existing Home Assistant client methods without modification

```rust
// Existing HomeAssistantClient interface (NO CHANGES)
impl HomeAssistantClient {
    /// Send command to Home Assistant via MQTT
    pub async fn send_command(&self, command: HomeAssistantCommand) -> Result<()>;

    /// Subscribe to Home Assistant events
    pub fn subscribe_events(&self) -> Receiver<HomeAssistantEvent>;
}
```

**Usage by Unified Source**:

```rust
// In SourceActivationContext::activate()
ha_client.send_command(HomeAssistantCommand::SelectSource {
    source: amplifier_source,
}).await?;
```

**Event Handling**:

```rust
// In ApplicationState::handle_homeassistant_event()
match event {
    HomeAssistantEvent::CommandAcknowledged { .. } => {
        // Log success, possibly update UI feedback
    }
    HomeAssistantEvent::CommandFailed { error, .. } => {
        // Log error, possibly show error on display
        tracing::error!("Source activation failed: {}", error);
    }
    // ... other events
}
```

---

### 4. ApplicationState Integration

**Modified Interface**: `src/controller/state.rs`

**New Methods**:

```rust
impl ApplicationState {
    /// Handle unified source selection (NEW)
    pub async fn select_unified_source(
        &mut self,
        source: &UnifiedSource,
        snapcast_client: &SnapcastClient,
        ha_client: &HomeAssistantClient,
    ) -> Result<()> {
        // Create activation context
        let context = SourceActivationContext::new(
            source,
            snapcast_client,
            ha_client,
            self.room.as_ref().map(|r| r.client_id.clone()).unwrap_or_default(),
            self.config
                .homeassistant
                .as_ref()
                .and_then(|ha| ha.amplifier.snapcast_amplifier_source)
                .unwrap_or(AmplifierSource::Spotify),
        );

        // Activate source (multi-step if Snapcast, single-step if amplifier)
        let result = context.activate().await?;

        // Update active source in collection
        self.unified_sources.set_active(source.clone());

        // Persist state
        self.save_unified_source_state()?;

        // Mark state change for display refresh
        self.last_state_change = Some(Instant::now());

        Ok(())
    }

    /// Navigate unified source selection up (NEW)
    pub fn unified_source_select_previous(&mut self) {
        self.unified_sources.select_previous();
    }

    /// Navigate unified source selection down (NEW)
    pub fn unified_source_select_next(&mut self) {
        self.unified_sources.select_next();
    }

    /// Get current UI-selected unified source (NEW)
    pub fn unified_source_selected(&self) -> Option<&UnifiedSource> {
        self.unified_sources.selected_source()
    }
}
```

**State Update Contract**:
- Unified source activation triggers `last_state_change` timestamp update
- Display refresh occurs if `current_page == PageView::UnifiedSourceSelection`
- State persistence happens synchronously after activation completes

---

## Event Flow Diagrams

### Source Activation Flow

```
┌────────────────────────────────────────────────────────────────────────┐
│ Hardware Event: Button Press (Confirm Selection)                       │
└────────────┬───────────────────────────────────────────────────────────┘
             │
             ▼
┌────────────────────────────────────────────────────────────────────────┐
│ controller/mapping.rs: handle_button_pressed(button_id)                │
│   - Identifies button as "confirm" button                              │
│   - Calls ApplicationState::select_unified_source()                    │
└────────────┬───────────────────────────────────────────────────────────┘
             │
             ▼
┌────────────────────────────────────────────────────────────────────────┐
│ controller/state.rs: select_unified_source(source, clients)            │
│   - Creates SourceActivationContext                                    │
│   - Calls context.activate()                                           │
└────────────┬───────────────────────────────────────────────────────────┘
             │
             ▼
┌────────────────────────────────────────────────────────────────────────┐
│ models/unified_source.rs: SourceActivationContext::activate()          │
│   - Matches on source type (Snapcast vs Amplifier)                     │
└────────────┬───────────────────────────────────────────────────────────┘
             │
   ┌─────────┴──────────┐
   │                    │
   ▼                    ▼
┌─────────────────┐  ┌──────────────────────────────────────────────────┐
│ Snapcast Stream │  │ Amplifier Input                                   │
└─────┬───────────┘  └──────┬───────────────────────────────────────────┘
      │                     │
      ▼                     ▼
┌──────────────────────────────────────┐  ┌─────────────────────────────┐
│ Step 1: homeassistant/client.rs      │  │ homeassistant/client.rs     │
│   send_command(SelectSource)         │  │   send_command(SelectSource)│
│   → MQTT IR command to switch amp    │  │   → MQTT IR command         │
└──────┬───────────────────────────────┘  └────────┬────────────────────┘
       │                                            │
       ▼                                            │
┌──────────────────────────────────────┐           │
│ Step 2: snapcast/client.rs           │           │
│   send_command(SetStream)            │           │
│   → JSON-RPC to Snapcast server      │           │
└──────┬───────────────────────────────┘           │
       │                                            │
       └───────────────┬────────────────────────────┘
                       │
                       ▼
┌────────────────────────────────────────────────────────────────────────┐
│ Return to ApplicationState::select_unified_source()                    │
│   - Update active source in UnifiedSourceCollection                    │
│   - Persist state to JSON file                                         │
│   - Mark state change timestamp                                        │
└────────────┬───────────────────────────────────────────────────────────┘
             │
             ▼
┌────────────────────────────────────────────────────────────────────────┐
│ Main Event Loop: Detect state change                                   │
│   - Check needs_screen_refresh()                                       │
│   - Call hardware/display.rs to render UnifiedSourceView               │
└────────────────────────────────────────────────────────────────────────┘
```

---

### State Synchronization Flow

```
┌────────────────────────────────────────────────────────────────────────┐
│ Snapcast Server: Stream list changes                                   │
│   (new stream added, stream removed, stream metadata updated)          │
└────────────┬───────────────────────────────────────────────────────────┘
             │
             ▼
┌────────────────────────────────────────────────────────────────────────┐
│ snapcast/client.rs: Receives SnapcastEvent::ServerReconnected          │
│   or SnapcastEvent::StreamUpdate                                       │
└────────────┬───────────────────────────────────────────────────────────┘
             │
             ▼
┌────────────────────────────────────────────────────────────────────────┐
│ controller/state.rs: handle_snapcast_event()                           │
│   - Calls unified_sources.update_snapcast_streams(new_streams)         │
└────────────┬───────────────────────────────────────────────────────────┘
             │
             ▼
┌────────────────────────────────────────────────────────────────────────┐
│ models/unified_source.rs: UnifiedSourceCollection::update_snapcast...()│
│   - Removes old Snapcast streams from sources list                     │
│   - Adds new Snapcast streams                                          │
│   - Preserves amplifier input sources unchanged                        │
└────────────┬───────────────────────────────────────────────────────────┘
             │
             ▼
┌────────────────────────────────────────────────────────────────────────┐
│ Main Event Loop: Refresh display if on UnifiedSourceSelection page     │
└────────────────────────────────────────────────────────────────────────┘
```

---

## Error Handling Contract

### Error Types

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum UnifiedSourceError {
    #[error("Source activation failed: {0}")]
    ActivationFailed(String),

    #[error("Snapcast stream not found: {0}")]
    StreamNotFound(String),

    #[error("Amplifier source selection failed: {0}")]
    AmplifierSelectionFailed(String),

    #[error("State persistence failed: {0}")]
    PersistenceFailed(String),

    #[error("Invalid configuration: {0}")]
    ConfigurationError(String),
}
```

### Error Handling Policy

1. **Activation Failures**: Log error, optionally show on display, do NOT crash application
2. **Persistence Failures**: Log warning, continue operation (state will be lost on restart)
3. **Configuration Errors**: Fail fast during startup with clear error message
4. **Network Errors**: Retry with exponential backoff (handled by existing Snapcast/HA clients)

### Example Error Handling

```rust
// In ApplicationState::select_unified_source()
match self.select_unified_source(source, snapcast_client, ha_client).await {
    Ok(_) => {
        tracing::info!("Source activated: {}", source);
    }
    Err(e) => {
        tracing::error!("Failed to activate source {}: {}", source, e);
        // Optionally: Show error on hardware display
        // self.show_error_message(&format!("Activation failed: {}", e));
    }
}
```

---

## Performance Contracts

### Latency Guarantees

| Operation | Maximum Latency | Measurement Point |
|-----------|----------------|-------------------|
| Source activation (total) | 2000ms | `activate()` call to completion |
| UI navigation (scroll) | 500ms | Button press to display update |
| State persistence | 100ms | `save_state()` completion |
| Collection rebuild | 50ms | `update_snapcast_streams()` |

### Resource Usage

| Resource | Constraint | Rationale |
|----------|-----------|-----------|
| Memory | < 1MB for source collection | Embedded controller constraint |
| CPU | < 5% during idle | Battery-powered device |
| Disk I/O | < 10 writes/minute | SSD wear leveling |

---

## Testing Contracts

### Unit Test Expectations

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unified_source_display_name() {
        let source = UnifiedSource::SnapcastStream {
            stream_id: "stream1".to_string(),
            name: "Test Stream".to_string(),
        };
        assert_eq!(source.display_name(), "Test Stream");
    }

    #[test]
    fn test_unified_source_serialization() {
        let source = UnifiedSource::AmplifierInput {
            source: AmplifierSource::Phono,
            name: "Turntable".to_string(),
        };

        let json = serde_json::to_string(&source).unwrap();
        let deserialized: UnifiedSource = serde_json::from_str(&json).unwrap();

        assert_eq!(source, deserialized);
    }

    #[test]
    fn test_source_collection_navigation() {
        let mut collection = UnifiedSourceCollection::new(vec![], AmplifierSource::Spotify);

        // Add test sources
        collection.update_snapcast_streams(vec![
            AudioStream { stream_id: "s1".into(), name: "Stream 1".into(), .. },
        ]);

        // Test navigation
        assert_eq!(collection.selected_index, 0);
        collection.select_next();
        assert_eq!(collection.selected_index, 1);
        collection.select_previous();
        assert_eq!(collection.selected_index, 0);
    }
}
```

### Integration Test Expectations

```rust
#[tokio::test]
async fn test_source_activation_snapcast_stream() {
    // Setup mock clients
    let snapcast_client = MockSnapcastClient::new();
    let ha_client = MockHomeAssistantClient::new();

    let source = UnifiedSource::SnapcastStream {
        stream_id: "test_stream".to_string(),
        name: "Test".to_string(),
    };

    let context = SourceActivationContext::new(
        &source,
        &snapcast_client,
        &ha_client,
        "client1".to_string(),
        AmplifierSource::Spotify,
    );

    // Activate and verify multi-step execution
    let result = context.activate().await.unwrap();

    // Verify amplifier switched to Snapcast source
    assert!(ha_client.received_select_source(AmplifierSource::Spotify));

    // Verify stream selection sent to Snapcast
    assert!(snapcast_client.received_set_stream("test_stream"));
}
```

---

## Backward Compatibility Contract

### Configuration Compatibility

```toml
# Old config (NO snapcast_amplifier_source field)
[homeassistant.amplifier]
power_entity = "switch.amplifier_power"
# → Defaults to AmplifierSource::Spotify

# New config (WITH snapcast_amplifier_source field)
[homeassistant.amplifier]
power_entity = "switch.amplifier_power"
snapcast_amplifier_source = "CD"
# → Uses specified source
```

### Feature Toggle

Unified source view can be disabled via configuration (future enhancement):

```toml
[features]
unified_source_view = true  # Enable unified view (default)
```

When disabled, application falls back to separate `StreamSelection` and `SourceSelection` pages.

---

## Summary

This contract specification defines:
- **Clear interface boundaries** between unified source module and existing subsystems
- **Event-driven integration** with Snapcast and Home Assistant clients
- **Error handling policy** for robust operation
- **Performance guarantees** aligned with spec success criteria
- **Testing expectations** for validation

All contracts maintain backward compatibility and follow established patterns in the codebase.
