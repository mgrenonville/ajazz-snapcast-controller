# Research: Unified Source View

**Feature**: 003-unified-source-view
**Date**: 2026-01-05
**Purpose**: Research technical decisions and best practices for implementing unified source abstraction

## Executive Summary

This document consolidates research findings for four key architectural decisions:
1. Unified source identification using Rust enum variants with associated data
2. Event-driven state synchronization leveraging existing notification infrastructure
3. Configuration extension with new `snapcast_amplifier_source` TOML field
4. Page navigation approach replacing existing separate pages with unified view

All decisions prioritize simplicity and reuse of existing patterns per project constitution.

---

## 1. Unified Source Identification Strategy

### Decision

**Use Rust enum variants with associated data**

```rust
pub enum UnifiedSource {
    SnapcastStream {
        stream_id: String,
        name: String
    },
    AmplifierInput {
        source: AmplifierSource,  // Reuses existing enum
        name: String
    },
}
```

### Rationale

- **Type Safety**: Rust's enum system provides compile-time guarantees that prevent mixing up Snapcast streams and amplifier inputs
- **Pattern Matching**: Enables exhaustive matching in activation logic, preventing missed cases
- **Zero-Cost Abstraction**: No runtime overhead compared to other approaches
- **Existing Patterns**: Follows established Rust idioms in the codebase (see `PageView`, `AmplifierSource` enums)
- **Serialization Support**: Serde supports tagged enum serialization out-of-the-box for JSON persistence

### Alternatives Considered

**Prefixed String IDs** (e.g., "snapcast:stream_id", "amplifier:Phono")
- ❌ Rejected: String parsing required, prone to runtime errors
- ❌ No compile-time validation of ID format
- ✓ Simpler serialization (just strings)

**UUID-based Identification**
- ❌ Rejected: Overcomplicated for small source count (5-15 sources total)
- ❌ Requires UUID generation and tracking
- ❌ Less human-readable in logs and debugging

### Implementation Notes

- Associate display name directly in enum to simplify UI rendering
- Implement `Display` trait for user-facing source names
- Implement `PartialEq` for state comparison (needed for "active source" highlighting)

---

## 2. State Synchronization Approach

### Decision

**Event-driven synchronization using existing Snapcast and Home Assistant event streams**

### Rationale

- **Existing Infrastructure**: Both `SnapcastEvent` and `HomeAssistantEvent` already provide real-time notifications
- **2-Second Requirement**: Event-driven updates meet latency requirement without polling overhead
- **Consistency**: Leverages same patterns used for current room volume/stream updates (see `controller/state.rs:handle_stream_changed`)
- **No New Dependencies**: Reuses tokio broadcast channels already in place

### Architecture

```rust
// In ApplicationState
pub fn handle_unified_source_event(&mut self, event: UnifiedSourceEvent) -> bool {
    match event {
        // Snapcast stream added/removed/updated
        UnifiedSourceEvent::SnapcastStreamsUpdated(streams) => {
            self.rebuild_unified_source_collection();
            self.current_page == PageView::UnifiedSourceSelection
        }
        // Amplifier source changed externally
        UnifiedSourceEvent::AmplifierSourceChanged(source) => {
            self.update_active_unified_source();
            self.current_page == PageView::UnifiedSourceSelection
        }
    }
}
```

### Alternatives Considered

**Polling-Based Synchronization**
- ❌ Rejected: Adds complexity, doesn't improve on event-driven approach
- ❌ Higher latency (poll interval vs instant notification)
- ✓ Simpler error handling (no event listener failures)

**Optimistic Local State with Eventual Consistency**
- ❌ Rejected: Risk of stale state display conflicts with 2-second update requirement
- ❌ Complexity in conflict resolution not justified for prototype
- ✓ Better user experience (instant feedback)

### Implementation Notes

- Unified source collection rebuilds whenever Snapcast streams update (`SnapcastEvent::ServerReconnected`, `StreamUpdate`)
- Active source tracking updated on `StreamChanged` and amplifier source selection commands
- Leverage existing `last_state_change` timestamp tracking in `ApplicationState`

---

## 3. Configuration Format for Snapcast Source Mapping

### Decision

**Add new TOML field `snapcast_amplifier_source` under `[homeassistant.amplifier]` section**

```toml
[homeassistant.amplifier]
power_entity = "switch.amplifier_power"
snapcast_amplifier_source = "Spotify"  # Which amplifier input Snapcast is connected to
```

### Rationale

- **Backward Compatibility**: Optional field, existing configs continue working (defaults to `Spotify` if not specified)
- **Discoverability**: Logical placement alongside other amplifier configuration
- **Validation**: Can validate value against `AmplifierSource::all()` during config parsing
- **Type Safety**: Deserializes directly to `AmplifierSource` enum via Serde

### Configuration Schema

```rust
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AmplifierConfig {
    pub power_entity: String,

    /// Which amplifier input source is connected to Snapcast output
    /// Defaults to Spotify if not specified
    #[serde(default = "default_snapcast_source")]
    pub snapcast_amplifier_source: AmplifierSource,
}

fn default_snapcast_source() -> AmplifierSource {
    AmplifierSource::Spotify
}
```

### Alternatives Considered

**Hardcoded Assumption**
- ❌ Rejected: Inflexible, doesn't support different hardware configurations
- ✓ Simplest implementation
- ❌ Violates "user configurable" principle from existing config pattern

**Auto-Detection Based on Naming**
- ❌ Rejected: Snapcast streams don't inherently know which amplifier input they're connected to
- ❌ Requires network probing or heuristics (overcomplicated)
- ✓ Zero configuration burden on users

### Implementation Notes

- Validation in `AmplifierConfig::validate()` ensures value is valid `AmplifierSource` variant
- Migration path: Existing configs without field use `Spotify` default (current typical setup)
- Documentation update in config.toml.example to explain field purpose

---

## 4. Page Navigation Integration

### Decision

**Replace `StreamSelection` and `SourceSelection` pages with new `UnifiedSourceSelection` page**

New navigation cycle: `Status → UnifiedSourceSelection → AmplifierControl → Status`

### Rationale

- **Simplified UX**: Single source selection interface aligns with feature goal (no mental model of "Snapcast vs amplifier")
- **Reduced Complexity**: Fewer pages to maintain, clearer navigation flow
- **Constitution Alignment**: Simplest solution that works (Principle III)
- **Backward Compatible Removal**: Old pages can be deprecated without config changes

### Navigation Flow

```rust
pub enum PageView {
    Status,
    UnifiedSourceSelection,  // NEW: replaces StreamSelection + SourceSelection
    AmplifierControl,
    Settings,
}

impl ApplicationState {
    pub fn next_page(&mut self) {
        self.current_page = match self.current_page {
            PageView::Status => PageView::UnifiedSourceSelection,
            PageView::UnifiedSourceSelection => {
                if self.amplifier.is_some() {
                    PageView::AmplifierControl
                } else {
                    PageView::Status
                }
            }
            PageView::AmplifierControl => PageView::Status,
            PageView::Settings => PageView::Status,
        };
    }
}
```

### Alternatives Considered

**Add as New 4th Page in Cycle**
- ❌ Rejected: Creates navigation clutter (4 pages to cycle through)
- ❌ Users still need to understand when to use which source selection page
- ✓ Preserves old pages for gradual migration

**Configurable via Settings**
- ❌ Rejected: Adds configuration complexity for feature that should "just work"
- ❌ Violates simplicity principle
- ✓ Maximum flexibility for power users

### Implementation Notes

- Remove `StreamSelection` and `SourceSelection` page rendering logic
- Update page navigation in `controller/mapping.rs` button handlers
- Unified source list shows all sources with visual distinction (optional: icons or prefixes like "🎵 Stream" vs "🔌 Input")
- Selection action triggers appropriate activation logic based on source type

---

## 5. Rust Best Practices Applied

### Enum Design with Associated Data

**Pattern**: Use enum variants with named fields for clarity

```rust
// ✓ GOOD: Named fields
pub enum UnifiedSource {
    SnapcastStream { stream_id: String, name: String },
    AmplifierInput { source: AmplifierSource, name: String },
}

// ❌ AVOID: Tuple variants less readable
pub enum UnifiedSource {
    SnapcastStream(String, String),  // Which is ID? Which is name?
    AmplifierInput(AmplifierSource, String),
}
```

### Async Multi-Step Coordination

**Pattern**: Use async functions with `?` operator for error propagation

```rust
pub async fn activate_source(
    &self,
    source: &UnifiedSource,
    snapcast_client: &SnapcastClient,
    ha_client: &HomeAssistantClient,
) -> Result<(), ActivationError> {
    match source {
        UnifiedSource::SnapcastStream { stream_id, .. } => {
            // Step 1: Switch amplifier to Snapcast input
            ha_client.select_source(self.config.snapcast_amplifier_source).await?;

            // Step 2: Select stream on Snapcast server
            snapcast_client.set_stream(stream_id).await?;
        }
        UnifiedSource::AmplifierInput { source, .. } => {
            // Single step: Switch amplifier input
            ha_client.select_source(*source).await?;
        }
    }
    Ok(())
}
```

### State Persistence with Serde

**Pattern**: Reuse existing JSON file persistence pattern from `amplifier_state.json`

```rust
#[derive(Debug, Serialize, Deserialize)]
struct UnifiedSourceState {
    active_source: UnifiedSource,
    last_updated: SystemTime,
}

impl ApplicationState {
    fn save_unified_source_state(&self) -> Result<(), Box<dyn Error>> {
        let state_file = dirs::config_dir()?.join("snapcast-controller/unified_source_state.json");

        let state = UnifiedSourceState {
            active_source: self.get_active_unified_source()?,
            last_updated: SystemTime::now(),
        };

        // Atomic write: temp file + rename
        let json = serde_json::to_string_pretty(&state)?;
        fs::write(&state_file, json)?;

        Ok(())
    }
}
```

---

## 6. TOML Configuration Extension

### Pattern

Follow existing configuration structure with optional fields and defaults:

```toml
[homeassistant]
broker_address = "192.168.1.100"
broker_port = 1883
ir_blaster_topic = "tasmota_17DD9F/cmnd/irsend"

[homeassistant.amplifier]
power_entity = "switch.amplifier_power"
snapcast_amplifier_source = "Spotify"  # NEW: Maps Snapcast to this amplifier input
```

### Backward Compatibility

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct AmplifierConfig {
    pub power_entity: String,

    // Optional field with default - existing configs without this field still parse
    #[serde(default = "default_snapcast_source")]
    pub snapcast_amplifier_source: AmplifierSource,
}
```

### Validation

```rust
impl AmplifierConfig {
    pub fn validate(&self) -> Result<()> {
        // Existing power_entity validation
        // ...

        // NEW: Validate snapcast_amplifier_source is valid variant
        // (Automatically validated by Serde deserialization to AmplifierSource enum)

        Ok(())
    }
}
```

---

## Dependencies Summary

| Dependency | Purpose | Already in Project | New/Existing |
|------------|---------|-------------------|--------------|
| serde | Enum serialization | ✓ Yes | Existing |
| serde_json | JSON state persistence | ✓ Yes | Existing |
| tokio | Async/await runtime | ✓ Yes | Existing |
| snapcast-control | Stream selection | ✓ Yes | Existing |
| rumqttc | MQTT amplifier commands | ✓ Yes | Existing |

**No new dependencies required** - all decisions leverage existing crate ecosystem.

---

## Performance Considerations

### Source Activation Latency Budget

Target: < 2 seconds for complete source switch

**Breakdown**:
- Amplifier input IR command: ~200ms (existing measurement from T044)
- Snapcast stream selection: ~300ms (JSON-RPC roundtrip)
- State update propagation: ~100ms (existing event handling)
- Display refresh: ~500ms (existing screen rendering)
- **Total**: ~1100ms (well under 2-second requirement)
- **Buffer**: 900ms for network variability and error handling

### UI Navigation Responsiveness

Target: < 500ms for scroll/selection feedback

**Approach**:
- Source list rendering uses existing display pipeline (already meets <500ms)
- Selection highlight update is synchronous (no network calls)
- Activation happens asynchronously after selection confirmed

---

## Testing Strategy (Optional per Constitution)

Per Principle I (Speed Over Perfection), comprehensive tests are not required for prototyping phase.

**Manual Testing Checklist** (when implemented):
- [ ] View unified source list showing both Snapcast streams and amplifier inputs
- [ ] Select Snapcast stream → verify amplifier switches to configured source + stream changes
- [ ] Select amplifier input → verify amplifier switches to correct input
- [ ] Restart application → verify last selected source is restored
- [ ] Navigate unified source view with hardware knob → verify smooth scrolling
- [ ] Disconnect Snapcast server → verify graceful handling in unified view

**Future Testing** (if needed for production):
- Unit tests for `UnifiedSource` enum serialization/deserialization
- Integration tests for source activation orchestration
- Property tests for state synchronization edge cases

---

## Open Questions Resolved

| Question | Answer | Rationale |
|----------|--------|-----------|
| How to identify sources uniquely? | Enum variants with associated data | Type safety, existing patterns |
| State sync approach? | Event-driven using existing streams | Meets latency, reuses infrastructure |
| Config format for Snapcast mapping? | New TOML field `snapcast_amplifier_source` | Backward compatible, validated |
| Page navigation integration? | Replace old pages with unified view | Simplest UX, clearest mental model |

All research topics from Phase 0 outline have been addressed with concrete decisions ready for Phase 1 design.

---

## Next Steps

Proceed to **Phase 1: Design & Contracts**
- Generate `data-model.md` with detailed struct/enum definitions
- Document API contracts in `contracts/unified-source-interface.md`
- Create `quickstart.md` developer onboarding guide
