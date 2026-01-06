# Implementation Plan: Unified Source View

**Branch**: `003-unified-source-view` | **Date**: 2026-01-05 | **Spec**: [spec.md](./spec.md)
**Input**: Feature specification from `/specs/003-unified-source-view/spec.md`

**Note**: This template is filled in by the `/speckit.plan` command. See `.specify/templates/commands/plan.md` for the execution workflow.

## Summary

Implement a unified source abstraction layer that presents both Snapcast audio streams and amplifier input sources as a single, cohesive list of selectable sources. Users will interact with a single "source selection" interface that automatically handles the complexity of switching between Snapcast streams (requiring amplifier input change + stream selection) and direct amplifier inputs (requiring only amplifier input change). The feature maintains state persistence across application restarts and provides seamless integration with existing Snapcast and Home Assistant control mechanisms.

## Technical Context

**Language/Version**: Rust stable (edition 2024) with async/await
**Primary Dependencies**: tokio (async runtime), serde/serde_json (serialization), snapcast-control (Snapcast JSON-RPC client), rumqttc (MQTT client), ajazz-sdk (USB hardware)
**Storage**: Local JSON file persistence (`~/.config/snapcast-controller/unified_source_state.json`)
**Testing**: cargo test (optional per constitution - tests not required for prototyping phase)
**Target Platform**: Linux embedded controller with USB HID hardware interface
**Project Type**: Single Rust binary with embedded modules (src/ structure)
**Performance Goals**: Source selection completes within 2 seconds, UI responds to navigation within 500ms
**Constraints**: Must maintain compatibility with existing Snapcast and Home Assistant integrations; no protocol changes allowed
**Scale/Scope**: Managing 5 amplifier sources + dynamic Snapcast streams (typically 2-10 streams)

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

### Pre-Research Gate ✓

**Principle I: Speed Over Perfection**
- ✓ Implementation will prioritize working abstraction layer over comprehensive edge case handling
- ✓ State persistence uses simple JSON file serialization (already established pattern)
- ✓ No extensive testing required initially - validate through manual testing with hardware

**Principle II: Incremental Delivery**
- ✓ Feature broken into 4 independent user stories (P1: view sources, P1: switch sources, P2: persistence, P2: navigation)
- ✓ Can deliver P1 stories first as MVP without P2 enhancements
- ✓ Each story independently testable and demonstrable

**Principle III: Simplicity First**
- ✓ Unified source abstraction uses enum-based approach (simple tagged union)
- ✓ Reuses existing communication channels (Snapcast JSON-RPC, MQTT for amplifier)
- ✓ No new frameworks or complex abstractions - direct implementation using existing patterns
- ✓ State management follows established patterns (see `amplifier_state.json` persistence)

**Violations**: None

**Complexity Justification**: Not applicable - no constitution violations

### Post-Design Gate

*(To be re-evaluated after Phase 1 design completion)*

## Project Structure

### Documentation (this feature)

```text
specs/003-unified-source-view/
├── plan.md              # This file (/speckit.plan command output)
├── research.md          # Phase 0 output (/speckit.plan command)
├── data-model.md        # Phase 1 output (/speckit.plan command)
├── quickstart.md        # Phase 1 output (/speckit.plan command)
├── contracts/           # Phase 1 output (/speckit.plan command)
│   └── unified-source-interface.md
└── tasks.md             # Phase 2 output (/speckit.tasks command - NOT created by /speckit.plan)
```

### Source Code (repository root)

```text
src/
├── models/              # [NEW] Unified source abstraction layer
│   └── unified_source.rs
├── services/            # [EXISTING] Business logic layer
├── controller/          # [EXISTING] Application state machine
│   ├── state.rs         # [MODIFY] Add unified source state management
│   ├── mapping.rs       # [MODIFY] Add unified source navigation handlers
│   └── mod.rs
├── snapcast/            # [EXISTING] Snapcast client integration
│   ├── client.rs
│   ├── types.rs
│   └── mod.rs
├── homeassistant/       # [EXISTING] Home Assistant MQTT integration
│   ├── client.rs
│   ├── types.rs         # AmplifierSource enum already defined here
│   ├── commands.rs
│   └── mod.rs
├── hardware/            # [EXISTING] USB HID hardware controller
│   ├── device.rs
│   ├── display.rs
│   ├── events.rs
│   └── mod.rs
└── main.rs              # [MODIFY] Wire up unified source view page handler

tests/                   # [OPTIONAL] Unit and integration tests
└── unified_source_tests.rs  # [NEW if testing] Unified source abstraction tests
```

**Structure Decision**: Single project structure is appropriate for this embedded controller application. The unified source abstraction will live in a new `src/models/` module, following Rust module organization conventions. Existing `controller/state.rs` will be extended to manage unified source state alongside existing Snapcast and amplifier state. The hardware display layer (`hardware/display.rs`) will be extended with a new page view for the unified source list.

## Complexity Tracking

> **Fill ONLY if Constitution Check has violations that must be justified**

Not applicable - no constitution violations identified.

## Phase 0: Research & Architecture Decisions

### Research Topics

1. **Unified Source Identification Strategy**
   - **Question**: How should we uniquely identify sources across Snapcast streams and amplifier inputs?
   - **Options**:
     - Prefixed IDs (e.g., "snapcast:stream_id", "amplifier:Phono")
     - Enum variants with associated data
     - UUID-based identification
   - **Research needed**: Evaluate trade-offs for state persistence, UI display, and state synchronization

2. **State Synchronization Approach**
   - **Question**: How should the unified source state reconcile with underlying Snapcast and amplifier states?
   - **Options**:
     - Polling-based synchronization
     - Event-driven updates from Snapcast/HA clients
     - Optimistic local state with eventual consistency
   - **Research needed**: Determine best approach for maintaining accuracy while meeting 2-second update requirement

3. **Configuration Format for Snapcast Source Mapping**
   - **Question**: How should configuration specify which amplifier input is connected to Snapcast?
   - **Options**:
     - New config.toml field: `snapcast_amplifier_source = "Spotify"`
     - Hardcoded assumption (least flexible)
     - Auto-detection based on naming conventions
   - **Research needed**: Choose configuration approach that balances simplicity with flexibility

4. **Page Navigation Integration**
   - **Question**: Should unified source view be a new page or replace existing pages?
   - **Options**:
     - New page in main navigation cycle (Status → Streams → **Unified** → Amplifier)
     - Replace both StreamSelection and SourceSelection pages
     - Configurable via settings
   - **Research needed**: Determine UX flow that maintains discoverability of existing features

### Dependencies & Best Practices

- **Rust Enum Design**: Best practices for enum variants with associated data (UnifiedSource enum)
- **Tokio Async Coordination**: Patterns for coordinating multi-step async operations (amplifier input switch → stream selection)
- **TOML Configuration**: Standard patterns for extending existing configuration with backward compatibility
- **State Persistence**: JSON file format and atomic write patterns (already established, validate reuse)

See [research.md](./research.md) for detailed findings.

## Phase 1: Design & Contracts

### Data Model

See [data-model.md](./data-model.md) for:
- `UnifiedSource` enum structure
- `UnifiedSourceCollection` aggregation
- `SourceActivationContext` for multi-step operations
- State persistence schema

### API Contracts

See [contracts/](./contracts/) for:
- Internal Rust trait definitions for unified source abstraction
- State machine transitions for source activation
- Integration contracts with existing Snapcast and HomeAssistant modules

### Quickstart Guide

See [quickstart.md](./quickstart.md) for developer onboarding:
- How to add new amplifier sources
- How unified source abstraction works
- State synchronization flow diagrams

## Phase 2: Task Breakdown

*Generated by `/speckit.tasks` command - see [tasks.md](./tasks.md)*

Tasks will be organized by user story priority:
1. **P1 Tasks**: Core unified source view and single-action switching
2. **P2 Tasks**: State persistence and navigation polish
3. **Integration Tasks**: Wire up hardware display and event handlers

## Post-Design Constitution Re-Check

### Design Review Against Constitution

**Principle I: Speed Over Perfection** ✓
- ✓ Design uses enum-based abstraction (simple, type-safe, no complex OOP)
- ✓ State persistence reuses existing JSON file pattern (no new libraries)
- ✓ Multi-step activation logic is straightforward async/await (no complex orchestration framework)
- ✓ No comprehensive tests in design - manual testing approach aligns with prototyping focus

**Principle II: Incremental Delivery** ✓
- ✓ Data model clearly separates P1 concerns (UnifiedSource, UnifiedSourceCollection) from P2 (state persistence)
- ✓ Phase 1 design enables delivering "view unified sources" + "switch sources" independently
- ✓ Persistence layer (`UnifiedSourceState`) is additive - P1 can ship without it
- ✓ Navigation integration (quickstart.md) documents how to deliver incrementally

**Principle III: Simplicity First** ✓
- ✓ No Repository pattern - direct state management in ApplicationState
- ✓ No complex state machines - simple enum matching in `activate()`
- ✓ Reuses existing clients (SnapcastClient, HomeAssistantClient) without wrappers
- ✓ Configuration extension is minimal (one optional TOML field with default)
- ✓ Page navigation simplifies by replacing two pages with one (reduces complexity)

**Violations**: None

**Design Approval**: ✅ **APPROVED** - Design adheres to all constitutional principles

## Notes & Open Questions

1. **Snapcast Source Mapping**: Need to decide on configuration field name and validation - addressed in research.md
2. **Error Handling**: How should we handle partial failures (e.g., amplifier input switches but stream selection fails)? - defer to implementation phase, use simple error logging initially
3. **Display Constraints**: Hardware screen size limits - need to validate scrolling implementation with actual device - can be addressed during P2 navigation tasks
4. **Backward Compatibility**: Should old pages (StreamSelection, SourceSelection) remain accessible? - research needed, likely keep as optional fallback during transition

## Success Criteria Mapping

Mapping spec success criteria to implementation deliverables:

- **SC-001** (Single unified interface): Delivered by new `UnifiedSourceView` page + `UnifiedSourceCollection`
- **SC-002** (Single button press switching): Delivered by `SourceActivationContext` orchestration logic
- **SC-003** (95% success rate, <2s): Monitored via existing latency tracking in `ApplicationState`
- **SC-004** (Transparent multi-step handling): Core responsibility of `SourceActivationContext::activate()`
- **SC-005** (100% restoration after restart): Delivered by unified source state persistence
- **SC-006** (Reduced cognitive load): Achieved by UI design hiding Snapcast-as-amplifier-source detail
- **SC-007** (<500ms navigation response): Validated with existing UI response time patterns
