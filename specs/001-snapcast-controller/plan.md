# Implementation Plan: Snapcast Controller Application

**Branch**: `001-snapcast-controller` | **Date**: 2025-12-12 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `/specs/001-snapcast-controller/spec.md`

**Note**: This template is filled in by the `/speckit.plan` command. See `.specify/templates/commands/plan.md` for the execution workflow.

## Summary

Build a desktop application that bridges a USB HID hardware controller (3 knobs, 6 buttons with screens, 3 page buttons) to a Snapcast multi-room audio server's JSON-RPC API. The application will enable users to control a single room's audio playback (volume, mute, stream selection) through physical hardware controls with real-time visual feedback on the controller's built-in screens.

## Technical Context

**Language/Version**: Rust (stable channel, edition 2024)
**Primary Dependencies**:
- `ajazz_sdk` - USB HID hardware controller communication
- `snapcast_control` - Snapcast server JSON-RPC API client
- `tokio` - Async runtime (industry standard, best ecosystem support)
- `serde` + `toml` - Configuration management (simple TOML deserialization)
- `anyhow` + `thiserror` - Error handling (anyhow for app-level, thiserror for module boundaries)

**Storage**: File-based configuration (TOML or JSON) for connection settings and room assignment
**Testing**: cargo test (tests optional per constitution - Speed Over Perfection principle)
**Target Platform**: Linux desktop (primary), Windows/macOS support to be determined based on ajazz_sdk platform support
**Project Type**: Single binary application (desktop daemon)
**Performance Goals**:
- Control latency <500ms (knob/button to server response)
- Screen update latency <2s for server state changes
- Application startup <3s

**Constraints**:
- USB HID device access requires appropriate permissions
- Network connectivity required for Snapcast server communication
- Single hardware controller per application instance
- Real-time event handling from both USB and network sources

**Scale/Scope**:
- Single room control per instance
- Support for 10+ simultaneous audio streams to select from
- Deployment: One instance per physical room with hardware controller

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

### Principle I: Speed Over Perfection ✅

**Status**: PASS

- Prototype-first approach: Start with basic USB and network communication, iterate based on testing
- Tests are optional: No TDD required per constitution; focus on getting hardware integration working
- Working solution priority: Get knobs/buttons controlling audio before optimizing code structure

### Principle II: Incremental Delivery ✅

**Status**: PASS

- User Story 1 (P1): Connect to server and room - Deliverable MVP (can verify connectivity)
- User Story 2 (P2): Monitor status on screens - Independent feature (displays work without control)
- User Story 3 (P3): Control playback - Builds on monitoring (knobs and buttons send commands)
- Each story is independently testable and demonstrable

### Principle III: Simplicity First ✅

**Status**: PASS

- Direct USB HID → Snapcast API bridge (no unnecessary abstraction layers)
- File-based config (no database overhead)
- Single binary deployment (no microservices or complex architecture)
- Minimal dependencies: Only what's necessary for USB HID and JSON-RPC communication
- No premature optimization: Start with synchronous patterns, add async only where blocking would hurt UX

**Overall Assessment**: Constitution compliant. Proceed to Phase 0 research.

## Project Structure

### Documentation (this feature)

```text
specs/001-snapcast-controller/
├── plan.md              # This file (/speckit.plan command output)
├── research.md          # Phase 0 output (/speckit.plan command)
├── data-model.md        # Phase 1 output (/speckit.plan command)
├── quickstart.md        # Phase 1 output (/speckit.plan command)
├── contracts/           # Phase 1 output (/speckit.plan command)
│   └── snapcast-api.md  # Snapcast JSON-RPC API subset we'll use
└── tasks.md             # Phase 2 output (/speckit.tasks command - NOT created by /speckit.plan)
```

### Source Code (repository root)

```text
src/
├── main.rs              # Application entry point, event loop
├── hardware/            # USB HID controller integration
│   ├── mod.rs
│   ├── device.rs        # ajazz_sdk wrapper
│   ├── events.rs        # Knob/button event handling
│   └── display.rs       # Screen rendering
├── snapcast/            # Snapcast server integration
│   ├── mod.rs
│   ├── client.rs        # snapcast_control wrapper
│   ├── types.rs         # Room, Stream, State models
│   └── commands.rs      # Volume, mute, stream assignment
├── config/              # Configuration management
│   ├── mod.rs
│   └── settings.rs      # Load/save TOML config
└── controller/          # Business logic
    ├── mod.rs
    ├── state.rs         # Application state management
    └── mapping.rs       # Hardware → Snapcast command mapping

Cargo.toml               # Project dependencies
config.toml.example      # Example configuration file

tests/                   # Optional - only if explicitly requested
├── integration/
└── unit/
```

**Structure Decision**: Single binary Rust application (Option 1 variant). The source is organized into four main modules:
1. **hardware**: Interfaces with the USB HID controller using ajazz_sdk
2. **snapcast**: Communicates with Snapcast server using snapcast_control
3. **config**: Manages persistent settings (server address, room assignment)
4. **controller**: Coordinates between hardware events and Snapcast commands

This structure keeps concerns separated while maintaining simplicity - no unnecessary abstractions or layers.

## Complexity Tracking

> **Fill ONLY if Constitution Check has violations that must be justified**

No constitution violations. All principles satisfied.


## Planning Phase Completion

### Phase 0: Research ✅

**Completed**: research.md created
**Decisions Made**:
- Async runtime: **Tokio** (industry standard, best ecosystem support)
- Configuration: **TOML + serde** (simple, human-friendly)
- Error handling: **anyhow + thiserror** (pragmatic balance)
- Architecture: **Event-driven** with tokio tasks and channels

All NEEDS CLARIFICATION items from Technical Context have been resolved.

### Phase 1: Design ✅

**Artifacts Created**:
1. **data-model.md**: Defines all core entities (RoomState, AudioStream, HardwareEvent, SnapcastEvent, ControllerCommand, ApplicationState)
2. **contracts/snapcast-api.md**: Documents Snapcast JSON-RPC API subset (Server.GetStatus, Client.SetVolume, Group.SetStream, notifications)
3. **quickstart.md**: User-facing guide for installation, configuration, and hardware controller usage

**Key Design Decisions**:
- Event-driven architecture with two event sources (USB HID, Snapcast network)
- Central controller state machine coordinating hardware ↔ server communication
- TOML configuration file at `~/.config/snapcast-controller/config.toml`
- Hardware button/knob mapping defined for user stories

### Constitution Re-Check ✅

**Post-Design Assessment**: ALL PRINCIPLES SATISFIED

- **Speed Over Perfection**: Simple architecture, deferred implementation details
- **Incremental Delivery**: User stories remain independently deliverable
- **Simplicity First**: Single binary, file config, standard Rust patterns, no over-engineering

**Overall Status**: ✅ READY FOR TASK GENERATION

---

## Next Steps

The planning phase is complete. Proceed to task generation:

```bash
/speckit.tasks
```

This will generate `tasks.md` with ordered, independently testable tasks organized by user story (P1, P2, P3).

