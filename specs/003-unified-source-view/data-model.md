# Data Model: Unified Source View

**Feature**: 003-unified-source-view
**Date**: 2026-01-05
**Purpose**: Define data structures for unified source abstraction layer

---

## Overview

This document specifies the data model for representing and managing a unified view of audio sources that combines Snapcast streams and amplifier inputs into a single abstraction. The model leverages Rust's type system for compile-time safety while maintaining simplicity through enum-based design.

---

## Core Entities

### 1. UnifiedSource (Primary Abstraction)

**Purpose**: Represents any selectable audio source, abstracting over Snapcast streams and amplifier inputs

**Location**: `src/models/unified_source.rs`

```rust
use serde::{Deserialize, Serialize};
use crate::homeassistant::types::AmplifierSource;

/// Unified representation of an audio source (Snapcast stream or amplifier input)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum UnifiedSource {
    /// Snapcast audio stream
    SnapcastStream {
        /// Unique stream identifier from Snapcast server
        stream_id: String,
        /// Display name (e.g., "Living Room", "Bluetooth")
        name: String,
    },

    /// Direct amplifier input (non-Snapcast)
    AmplifierInput {
        /// Amplifier source enum (Phono, CD, etc.)
        source: AmplifierSource,
        /// Display name (e.g., "Phono", "CD Player")
        name: String,
    },
}
```

**Field Descriptions**:

| Field | Type | Purpose | Validation |
|-------|------|---------|------------|
| `stream_id` | `String` | Snapcast stream unique identifier | Non-empty, matches Snapcast server stream ID |
| `name` | `String` | User-facing display name | Non-empty, max 30 chars (display constraint) |
| `source` | `AmplifierSource` | Amplifier input enum variant | Must be valid `AmplifierSource` variant |

**Methods**:

```rust
impl UnifiedSource {
    /// Get display name for UI rendering
    pub fn display_name(&self) -> &str {
        match self {
            UnifiedSource::SnapcastStream { name, .. } => name,
            UnifiedSource::AmplifierInput { name, .. } => name,
        }
    }

    /// Check if this is the Snapcast-connected amplifier source
    pub fn is_snapcast_source(&self, config_snapcast_source: AmplifierSource) -> bool {
        match self {
            UnifiedSource::SnapcastStream { .. } => false,
            UnifiedSource::AmplifierInput { source, .. } => *source == config_snapcast_source,
        }
    }

    /// Get a unique identifier for equality comparison
    pub fn identifier(&self) -> String {
        match self {
            UnifiedSource::SnapcastStream { stream_id, .. } => format!("snapcast:{}", stream_id),
            UnifiedSource::AmplifierInput { source, .. } => format!("amplifier:{:?}", source),
        }
    }
}

impl std::fmt::Display for UnifiedSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}
```

**Serialization Example**:

```json
// Snapcast stream
{
  "type": "snapcast_stream",
  "stream_id": "stream_spotify",
  "name": "Spotify"
}

// Amplifier input
{
  "type": "amplifier_input",
  "source": "Phono",
  "name": "Turntable"
}
```

---

### 2. UnifiedSourceCollection (Aggregation)

**Purpose**: Manages the complete list of available sources and tracks active selection

**Location**: `src/models/unified_source.rs`

```rust
use std::collections::HashMap;

/// Collection of all available unified sources
#[derive(Debug, Clone)]
pub struct UnifiedSourceCollection {
    /// All available sources (Snapcast streams + amplifier inputs)
    sources: Vec<UnifiedSource>,

    /// Currently active source (if any)
    active_source: Option<UnifiedSource>,

    /// Amplifier source configured for Snapcast output
    snapcast_amplifier_source: AmplifierSource,

    /// Source index for UI selection/navigation
    selected_index: usize,
}
```

**Field Descriptions**:

| Field | Type | Purpose |
|-------|------|---------|
| `sources` | `Vec<UnifiedSource>` | Ordered list of all available sources |
| `active_source` | `Option<UnifiedSource>` | Currently playing source (None if unknown) |
| `snapcast_amplifier_source` | `AmplifierSource` | Config value: which amplifier input Snapcast uses |
| `selected_index` | `usize` | UI cursor position for navigation |

**Methods**:

```rust
impl UnifiedSourceCollection {
    /// Create a new collection from Snapcast streams and amplifier config
    pub fn new(
        snapcast_streams: Vec<AudioStream>,
        snapcast_amplifier_source: AmplifierSource,
    ) -> Self {
        let mut sources = Vec::new();

        // Add Snapcast streams
        for stream in snapcast_streams {
            sources.push(UnifiedSource::SnapcastStream {
                stream_id: stream.stream_id,
                name: stream.name,
            });
        }

        // Add all amplifier sources EXCEPT the one connected to Snapcast
        // (Snapcast streams already represent that source's content)
        for amp_source in AmplifierSource::all() {
            if amp_source != snapcast_amplifier_source {
                sources.push(UnifiedSource::AmplifierInput {
                    source: amp_source,
                    name: amp_source.display_name().to_string(),
                });
            }
        }

        Self {
            sources,
            active_source: None,
            snapcast_amplifier_source,
            selected_index: 0,
        }
    }

    /// Update available Snapcast streams (called on stream list changes)
    pub fn update_snapcast_streams(&mut self, streams: Vec<AudioStream>) {
        // Remove old Snapcast streams
        self.sources.retain(|s| !matches!(s, UnifiedSource::SnapcastStream { .. }));

        // Add new Snapcast streams at the beginning
        let mut new_streams: Vec<UnifiedSource> = streams
            .into_iter()
            .map(|stream| UnifiedSource::SnapcastStream {
                stream_id: stream.stream_id,
                name: stream.name,
            })
            .collect();

        new_streams.extend(self.sources.drain(..));
        self.sources = new_streams;

        // Reset selected_index if out of bounds
        if self.selected_index >= self.sources.len() {
            self.selected_index = 0;
        }
    }

    /// Set the currently active source
    pub fn set_active(&mut self, source: UnifiedSource) {
        self.active_source = Some(source);
    }

    /// Get currently active source
    pub fn active_source(&self) -> Option<&UnifiedSource> {
        self.active_source.as_ref()
    }

    /// Get all sources
    pub fn all_sources(&self) -> &[UnifiedSource] {
        &self.sources
    }

    /// Get currently selected source (for UI navigation)
    pub fn selected_source(&self) -> Option<&UnifiedSource> {
        self.sources.get(self.selected_index)
    }

    /// Navigate selection up (wraps around)
    pub fn select_previous(&mut self) {
        if self.sources.is_empty() {
            return;
        }
        if self.selected_index == 0 {
            self.selected_index = self.sources.len() - 1;
        } else {
            self.selected_index -= 1;
        }
    }

    /// Navigate selection down (wraps around)
    pub fn select_next(&mut self) {
        if self.sources.is_empty() {
            return;
        }
        self.selected_index = (self.selected_index + 1) % self.sources.len();
    }

    /// Confirm selection (activate the currently selected source)
    pub fn confirm_selection(&self) -> Option<&UnifiedSource> {
        self.selected_source()
    }
}
```

---

### 3. SourceActivationContext (Multi-Step Orchestration)

**Purpose**: Encapsulates the logic for activating a unified source, handling multi-step operations

**Location**: `src/models/unified_source.rs`

```rust
use crate::snapcast::client::SnapcastClient;
use crate::homeassistant::client::HomeAssistantClient;
use crate::homeassistant::types::HomeAssistantCommand;
use crate::snapcast::types::SnapcastCommand;
use anyhow::Result;

/// Context for activating a unified source
pub struct SourceActivationContext<'a> {
    /// Target source to activate
    target_source: &'a UnifiedSource,

    /// Snapcast client for stream selection
    snapcast_client: &'a SnapcastClient,

    /// Home Assistant client for amplifier control
    ha_client: &'a HomeAssistantClient,

    /// Snapcast client ID (for stream selection commands)
    snapcast_client_id: String,

    /// Amplifier source configured for Snapcast
    snapcast_amplifier_source: AmplifierSource,
}
```

**Methods**:

```rust
impl<'a> SourceActivationContext<'a> {
    /// Create a new activation context
    pub fn new(
        target_source: &'a UnifiedSource,
        snapcast_client: &'a SnapcastClient,
        ha_client: &'a HomeAssistantClient,
        snapcast_client_id: String,
        snapcast_amplifier_source: AmplifierSource,
    ) -> Self {
        Self {
            target_source,
            snapcast_client,
            ha_client,
            snapcast_client_id,
            snapcast_amplifier_source,
        }
    }

    /// Activate the target source
    ///
    /// For Snapcast streams:
    ///   1. Switch amplifier to Snapcast input (if not already)
    ///   2. Select stream on Snapcast server
    ///
    /// For amplifier inputs:
    ///   1. Switch amplifier to specified input
    pub async fn activate(&self) -> Result<ActivationResult> {
        match self.target_source {
            UnifiedSource::SnapcastStream { stream_id, name } => {
                // Multi-step activation for Snapcast streams
                tracing::info!("Activating Snapcast stream: {}", name);

                // Step 1: Switch amplifier to Snapcast input
                self.ha_client
                    .send_command(HomeAssistantCommand::SelectSource {
                        source: self.snapcast_amplifier_source,
                    })
                    .await?;

                // Step 2: Select the stream on Snapcast server
                self.snapcast_client
                    .send_command(SnapcastCommand::SetStream {
                        client_id: self.snapcast_client_id.clone(),
                        stream_id: stream_id.clone(),
                    })
                    .await?;

                Ok(ActivationResult::SnapcastStreamActivated {
                    stream_id: stream_id.clone(),
                })
            }

            UnifiedSource::AmplifierInput { source, name } => {
                // Single-step activation for amplifier inputs
                tracing::info!("Activating amplifier input: {}", name);

                self.ha_client
                    .send_command(HomeAssistantCommand::SelectSource { source: *source })
                    .await?;

                Ok(ActivationResult::AmplifierInputActivated { source: *source })
            }
        }
    }
}

/// Result of source activation
#[derive(Debug)]
pub enum ActivationResult {
    /// Snapcast stream successfully activated
    SnapcastStreamActivated { stream_id: String },

    /// Amplifier input successfully activated
    AmplifierInputActivated { source: AmplifierSource },
}
```

---

### 4. UnifiedSourceState (Persistence)

**Purpose**: Serializable state for persisting active source across application restarts

**Location**: `src/models/unified_source.rs`

```rust
use std::time::SystemTime;

/// Persistent state for unified source selection
#[derive(Debug, Serialize, Deserialize)]
pub struct UnifiedSourceState {
    /// Currently active unified source
    pub active_source: UnifiedSource,

    /// Timestamp of last update
    #[serde(with = "systemtime_serde")]
    pub last_updated: SystemTime,
}

/// Custom serde module for SystemTime
mod systemtime_serde {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::time::{SystemTime, UNIX_EPOCH};

    pub fn serialize<S>(time: &SystemTime, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let duration = time.duration_since(UNIX_EPOCH).unwrap();
        duration.as_secs().serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<SystemTime, D::Error>
    where
        D: Deserializer<'de>,
    {
        let secs = u64::deserialize(deserializer)?;
        Ok(UNIX_EPOCH + std::time::Duration::from_secs(secs))
    }
}
```

**Persistence Methods** (in `ApplicationState`):

```rust
impl ApplicationState {
    /// Save unified source state to file
    pub fn save_unified_source_state(&self) -> Result<()> {
        if let Some(active_source) = &self.unified_sources.active_source() {
            let state = UnifiedSourceState {
                active_source: active_source.clone(),
                last_updated: SystemTime::now(),
            };

            let state_file = dirs::config_dir()
                .context("Failed to get config directory")?
                .join("snapcast-controller/unified_source_state.json");

            // Create parent directory if needed
            if let Some(parent) = state_file.parent() {
                fs::create_dir_all(parent)?;
            }

            // Write JSON file
            let json = serde_json::to_string_pretty(&state)?;
            fs::write(state_file, json)?;

            tracing::debug!("Saved unified source state: {}", active_source);
        }

        Ok(())
    }

    /// Load unified source state from file
    pub fn load_unified_source_state(&mut self) -> Result<()> {
        let state_file = dirs::config_dir()
            .context("Failed to get config directory")?
            .join("snapcast-controller/unified_source_state.json");

        if !state_file.exists() {
            tracing::debug!("No unified source state file found");
            return Ok(());
        }

        let json = fs::read_to_string(&state_file)?;
        let state: UnifiedSourceState = serde_json::from_str(&json)?;

        // Restore active source
        self.unified_sources.set_active(state.active_source.clone());

        tracing::info!("Restored unified source state: {}", state.active_source);

        Ok(())
    }
}
```

**Persistence File Location**: `~/.config/snapcast-controller/unified_source_state.json`

**Example Persisted State**:

```json
{
  "active_source": {
    "type": "snapcast_stream",
    "stream_id": "stream_spotify",
    "name": "Spotify"
  },
  "last_updated": 1704441600
}
```

---

## State Machine Transitions

### Source Activation State Diagram

```
┌─────────────────────────────────────────────────────────────────┐
│ User Confirms Selection on Unified Source View                  │
└─────────────────────┬───────────────────────────────────────────┘
                      │
                      ▼
         ┌────────────────────────┐
         │ Get Selected Source    │
         └────────┬───────────────┘
                  │
      ┌───────────┴──────────────┐
      │                          │
      ▼                          ▼
┌──────────────┐        ┌──────────────────┐
│ Snapcast     │        │ Amplifier Input  │
│ Stream?      │        │ (non-Snapcast)?  │
└──────┬───────┘        └─────────┬────────┘
       │                          │
       │                          │
       ▼                          ▼
┌─────────────────────┐   ┌──────────────────────┐
│ Step 1:             │   │ Single Step:         │
│ Switch amplifier    │   │ Switch amplifier     │
│ to Snapcast input   │   │ to selected input    │
│ (IR command)        │   │ (IR command)         │
└──────┬──────────────┘   └──────┬───────────────┘
       │                          │
       ▼                          │
┌─────────────────────┐          │
│ Step 2:             │          │
│ Select stream on    │          │
│ Snapcast server     │          │
│ (JSON-RPC)          │          │
└──────┬──────────────┘          │
       │                          │
       └──────────┬───────────────┘
                  │
                  ▼
         ┌────────────────────────┐
         │ Update Active Source   │
         │ in Collection          │
         └────────┬───────────────┘
                  │
                  ▼
         ┌────────────────────────┐
         │ Persist State to File  │
         └────────┬───────────────┘
                  │
                  ▼
         ┌────────────────────────┐
         │ Trigger Display Refresh│
         └────────────────────────┘
```

---

## Integration with ApplicationState

### New Fields in `controller/state.rs`

```rust
pub struct ApplicationState {
    // ... existing fields ...

    /// Unified source collection (NEW)
    pub unified_sources: UnifiedSourceCollection,

    /// Whether unified source view is enabled (NEW)
    pub unified_view_enabled: bool,
}
```

### Initialization

```rust
impl ApplicationState {
    pub fn new(config: ConnectionSettings) -> Self {
        // Determine Snapcast amplifier source from config
        let snapcast_amplifier_source = config
            .homeassistant
            .as_ref()
            .and_then(|ha| ha.amplifier.snapcast_amplifier_source)
            .unwrap_or(AmplifierSource::Spotify);  // Default

        let unified_sources = UnifiedSourceCollection::new(
            Vec::new(),  // Will be populated on server connection
            snapcast_amplifier_source,
        );

        Self {
            // ... existing initialization ...
            unified_sources,
            unified_view_enabled: true,  // Enable unified view by default
        }
    }
}
```

---

## Configuration Schema Extension

### TOML Addition

```toml
[homeassistant.amplifier]
power_entity = "switch.amplifier_power"
snapcast_amplifier_source = "Spotify"  # NEW: Which amp input Snapcast uses
```

### Rust Config Structure

```rust
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AmplifierPowerEntityConfig {
    pub power_entity: String,

    /// Which amplifier input source Snapcast is connected to
    /// Defaults to Spotify if not specified
    #[serde(default = "default_snapcast_source")]
    pub snapcast_amplifier_source: AmplifierSource,
}

fn default_snapcast_source() -> AmplifierSource {
    AmplifierSource::Spotify
}
```

---

## Validation Rules

### UnifiedSource Validation

1. **Display name**: Non-empty, max 30 characters
2. **Stream ID**: Non-empty for Snapcast streams
3. **Amplifier source**: Must be valid `AmplifierSource` variant

### UnifiedSourceCollection Validation

1. **No duplicate sources**: Each source identifier must be unique
2. **At least one source**: Collection should not be empty (warn if empty, don't error)
3. **Active source exists**: If set, active source must be in sources list

### Configuration Validation

1. **snapcast_amplifier_source**: Must be valid `AmplifierSource` variant
2. **Backward compatibility**: Field is optional, defaults to `Spotify`

---

## Summary

This data model provides:
- **Type-safe abstraction** via Rust enums with compile-time guarantees
- **Simple state management** with clear ownership and mutation patterns
- **Persistence support** using established JSON serialization patterns
- **Integration points** with existing Snapcast and Home Assistant subsystems
- **UI-friendly structures** for navigation and display rendering

All structures follow Rust conventions and project patterns established in earlier features.
