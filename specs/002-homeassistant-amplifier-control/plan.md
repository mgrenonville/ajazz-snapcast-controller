# Implementation Plan: Home Assistant Amplifier Control

**Branch**: `002-homeassistant-amplifier-control` | **Date**: 2025-12-22 | **Spec**: [spec.md](./spec.md)
**Input**: Feature specification from `/specs/002-homeassistant-amplifier-control/spec.md`

**Note**: This template is filled in by the `/speckit.plan` command. See `.specify/templates/commands/plan.md` for the execution workflow.

## Summary

Add Home Assistant integration to the Ajazz hardware controller, enabling users to monitor and control amplifier power state and input sources directly from the controller's hardware interface. The feature integrates with existing Snapcast controller pages, adding a new "Amplifier Control" page accessible via page navigation buttons.

## Technical Context

**Language/Version**: Rust (stable channel, edition 2021 or later)
**Primary Dependencies**: tokio (async runtime), serde/toml (config), ajazz-sdk (hardware), rumqttc (MQTT client)
**Storage**: File-based configuration (TOML files in ~/.config or similar)
**Testing**: Optional per constitution - cargo test when explicitly needed
**Target Platform**: Linux/Unix systems with USB HID support
**Project Type**: single (monolithic Rust binary)
**Performance Goals**: <2 second display updates for state changes, <500ms command acknowledgment
**Constraints**: Must integrate with existing Snapcast control flow, share hardware display with existing pages, maintain <100ms network latency to Home Assistant
**Scale/Scope**: Single-user controller, 1 amplifier, ~5-10 input sources, concurrent with Snapcast control

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

- P1 (Monitor/Control Amplifier Power) is standalone MVP - can test power toggle without source selection
- P2 (Select Input Sources) adds value incrementally - can test source switching independently
- P3 (Page Navigation) is pure UX convenience - can demonstrate Snapcast + Amplifier as separate workflows before integration
- Each story has clear acceptance criteria and can be demo'd independently

**Action**: Implement stories in priority order (P1 → P2 → P3)

### Principle III: Simplicity First ✅ PASS

**Assessment**: Feature chooses direct implementations over abstractions.

- Reuses existing PageView enum and state machine patterns from Snapcast integration
- Direct Home Assistant API calls via simple client, no ORM or complex abstraction layers
- Configuration via existing TOML file structure (extend ConnectionSettings)
- No feature flags, version compatibility layers, or future-proofing
- Shares existing display manager and hardware event loop

**Action**: Use simplest approach - extend existing modules, avoid new frameworks

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
│   ├── client.rs             # Home Assistant API client
│   ├── types.rs              # Amplifier state, entity types
│   └── commands.rs           # Power toggle, source selection commands
└── main.rs                   # Add Home Assistant connection task, event routing

tests/                        # Optional per constitution
└── integration/              # If needed for production readiness
    └── homeassistant_test.rs
```

**Structure Decision**: Single project structure maintained. New `homeassistant/` module mirrors `snapcast/` pattern (client, types, commands). Extends existing `controller/state.rs` for amplifier state management. Reuses `hardware/display.rs` for new page layouts.

## Complexity Tracking

*No complexity violations - constitution check passed cleanly*
