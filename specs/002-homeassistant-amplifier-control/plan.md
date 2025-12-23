# Implementation Plan: Home Assistant Amplifier Control

**Branch**: `002-homeassistant-amplifier-control` | **Date**: 2025-12-22 | **Spec**: [spec.md](./spec.md)
**Input**: Feature specification from `/specs/002-homeassistant-amplifier-control/spec.md`

**Note**: This template is filled in by the `/speckit.plan` command. See `.specify/templates/commands/plan.md` for the execution workflow.

## Summary

Add Home Assistant integration to the Ajazz hardware controller, enabling users to monitor amplifier power state and control amplifier functions (source selection, volume) via IR commands sent through an MQTT-connected IR Blaster. The controller maintains local state for source selection and sends RC5 IR commands via MQTT to a Tasmota IR Blaster device. The feature integrates with existing Snapcast controller pages, adding a new "Amplifier Control" page accessible via page navigation buttons.

## Technical Context

**Language/Version**: Rust (stable channel, edition 2021 or later)
**Primary Dependencies**: tokio (async runtime), serde/toml (config), ajazz-sdk (hardware), rumqttc (MQTT client), serde_json (IR command payloads)
**Storage**: File-based configuration (TOML files in ~/.config or similar) for MQTT settings and IR Blaster topic; local state persistence for selected source
**Testing**: Optional per constitution - cargo test when explicitly needed
**Target Platform**: Linux/Unix systems with USB HID support
**Project Type**: single (monolithic Rust binary)
**Performance Goals**: <200ms IR command publishing, <2 second power state updates, <100ms volume command response to knob rotation
**Constraints**: Must integrate with existing Snapcast control flow, share hardware display with existing pages, maintain <100ms network latency to MQTT broker, IR commands are one-way (no feedback from amplifier)
**Scale/Scope**: Single-user controller, 1 amplifier, 5 input sources (Phono, CD, Spotify, Source 4, Source 5), volume control via rotary knob, concurrent with Snapcast control

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

### Principle I: Speed Over Perfection ✅ PASS

**Assessment**: Feature design prioritizes working integration over comprehensive testing or perfect architecture.

- Feature broken into 3 independently deliverable stories (P1: power control, P2: source selection, P3: page navigation)
- P1 provides immediate value as MVP without requiring P2 or P3
- Tests marked as optional, can validate manually with hardware
- No requirement for extensive error handling beyond basic user feedback

**Action**: Proceed with rapid prototyping approach

### Principle II: Incremental Delivery ✅ PASS

**Assessment**: User stories are independently testable and deliverable.

- P1 (Monitor/Control Amplifier Power) is standalone MVP - can test power toggle without source selection or volume control
- P2 (Select Input Sources) adds value incrementally - can test source switching independently with local state tracking
- P2 (Adjust Amplifier Volume) adds value incrementally - can test volume knob independently by sending IR commands
- P3 (Page Navigation) is pure UX convenience - can demonstrate Snapcast + Amplifier as separate workflows before integration
- Each story has clear acceptance criteria and can be demo'd independently

**Action**: Implement stories in priority order (P1 → P2 stories → P3)

### Principle III: Simplicity First ✅ PASS

**Assessment**: Feature chooses direct implementations over abstractions.

- Reuses existing PageView enum and state machine patterns from Snapcast integration
- Direct MQTT publish for IR commands via simple client, no complex abstraction layers
- Local state tracking for source selection (simple enum persisted to file), no complex synchronization
- Configuration via existing TOML file structure (extend ConnectionSettings with IR Blaster topic)
- Hardcoded RC5 IR command codes (no IR protocol abstraction needed for 5 sources + 2 volume commands)
- No feature flags, version compatibility layers, or future-proofing
- Shares existing display manager and hardware event loop
- Reuses existing rotary knob event handling from hardware layer

**Action**: Use simplest approach - extend existing modules, avoid new frameworks, keep IR commands as simple JSON payloads

### Overall Gate Status: ✅ PASS

No violations detected. Feature aligns with all three constitution principles. Proceed to Phase 0 research.

## Project Structure

### Documentation (this feature)

```text
specs/002-homeassistant-amplifier-control/
├── plan.md              # This file (/speckit.plan command output)
├── research.md          # Phase 0 output (/speckit.plan command)
├── data-model.md        # Phase 1 output (/speckit.plan command)
├── quickstart.md        # Phase 1 output (/speckit.plan command)
├── contracts/           # Phase 1 output (/speckit.plan command)
│   └── homeassistant-api.md  # Home Assistant API interface definition
└── tasks.md             # Phase 2 output (/speckit.tasks command - NOT created by /speckit.plan)
```

### Source Code (repository root)

```text
src/
├── config/
│   ├── mod.rs
│   └── settings.rs           # Extend with Home Assistant connection settings
├── controller/
│   ├── mod.rs
│   ├── mapping.rs
│   └── state.rs              # Add amplifier state, extend PageView enum
├── hardware/
│   ├── mod.rs
│   ├── device.rs
│   ├── display.rs            # Add amplifier control page layouts
│   └── events.rs             # Reuse existing events, may add amplifier-specific
├── snapcast/                 # Existing - no changes
│   ├── mod.rs
│   ├── client.rs
│   └── types.rs
├── homeassistant/            # NEW MODULE
│   ├── mod.rs
│   ├── client.rs             # MQTT client for Home Assistant and IR Blaster
│   ├── types.rs              # Amplifier state, IR command types
│   ├── commands.rs           # IR command builders (source, volume)
│   └── ir_codes.rs           # RC5 IR code constants
└── main.rs                   # Add Home Assistant connection task, event routing

tests/                        # Optional per constitution
└── integration/              # If needed for production readiness
    └── homeassistant_test.rs
```

**Structure Decision**: Single project structure maintained. New `homeassistant/` module mirrors `snapcast/` pattern (client, types, commands). Adds `ir_codes.rs` for RC5 command constants. Extends existing `controller/state.rs` for amplifier state management with local state tracking for selected source. Reuses `hardware/display.rs` for new page layouts. Reuses existing hardware event handling for rotary knob volume control.

## Complexity Tracking

*No complexity violations - constitution check passed cleanly*
