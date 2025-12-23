---

description: "Task list for Home Assistant Amplifier Control feature implementation"
---

# Tasks: Home Assistant Amplifier Control

**Input**: Design documents from `/specs/002-homeassistant-amplifier-control/`
**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/mqtt-api.md, quickstart.md

**Tests**: Tests are OPTIONAL per project constitution. No test tasks generated unless explicitly requested.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

## Path Conventions

- Single Rust project: `src/` at repository root
- Paths assume project structure from plan.md

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Add MQTT dependencies and basic configuration structure

- [x] T001 Add rumqttc dependency to Cargo.toml (version 0.24)
- [x] T002 [P] Create src/homeassistant/ module directory
- [x] T003 [P] Add homeassistant module declaration to src/main.rs

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core infrastructure that MUST be complete before ANY user story can be implemented

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

- [x] T004 Create HomeAssistantConfig struct in src/config/settings.rs (add ir_blaster_topic field)
- [x] T005 [P] Create AmplifierPowerEntityConfig struct in src/config/settings.rs
- [x] T006 Extend ConnectionSettings with homeassistant field in src/config/settings.rs
- [x] T007 [P] Create RC5 IR code constants in src/homeassistant/ir_codes.rs (sources: Phono 0xC01, CD 0xC02, Spotify 0xC03, Source4 0xC04, Source5 0xC05; volume: Up 0xC10, Down 0xC11)
- [x] T008 [P] Create AmplifierSource enum in src/homeassistant/types.rs (Phono, CD, Spotify, Source4, Source5)
- [x] T009 [P] Create IrCommand struct in src/homeassistant/types.rs (protocol, bits, data, repeat)
- [x] T010 [P] Create AmplifierState struct in src/homeassistant/types.rs (power: bool, selected_source: AmplifierSource, last_updated: timestamp)
- [x] T011 [P] Create EntityAvailability enum in src/homeassistant/types.rs
- [x] T012 [P] Create HomeAssistantConnection struct in src/homeassistant/types.rs
- [x] T013 [P] Create HomeAssistantEvent enum in src/homeassistant/types.rs
- [x] T014 [P] Create HomeAssistantCommand enum in src/homeassistant/types.rs (TogglePower, SelectSource, VolumeUp, VolumeDown)
- [x] T015 Create IR command builder functions in src/homeassistant/commands.rs (build_source_command, build_volume_command)
- [x] T016 Create homeassistant module exports in src/homeassistant/mod.rs
- [x] T017 Create MqttClient struct skeleton in src/homeassistant/client.rs
- [x] T018 Implement MQTT connection logic in src/homeassistant/client.rs
- [x] T019 Implement topic subscription logic for power state in src/homeassistant/client.rs
- [x] T020 Implement IR command publishing logic (publish to configurable ir_blaster_topic) in src/homeassistant/client.rs
- [x] T021 Implement error handling and reconnection with exponential backoff in src/homeassistant/client.rs
- [x] T022 Extend ApplicationState with homeassistant_connected field in src/controller/state.rs
- [x] T023 Extend ApplicationState with amplifier field (AmplifierState with local source tracking) in src/controller/state.rs
- [x] T024 Add PageView::AmplifierControl variant to src/controller/state.rs
- [x] T025 Implement ApplicationState::handle_homeassistant_event method in src/controller/state.rs
- [x] T026 [P] Implement local state persistence for selected_source (save/load from file) in src/controller/state.rs

**Checkpoint**: Foundation ready - user story implementation can now begin in parallel

---

## Phase 3: User Story 1 - Monitor and Control Amplifier Power (Priority: P1) 🎯 MVP

**Goal**: Enable users to view amplifier power state on hardware display and toggle power on/off via controller button

**Independent Test**: Connect controller to Home Assistant, view amplifier power state on display, toggle power via button, verify display updates and physical amplifier responds within 2 seconds

### Implementation for User Story 1

- [x] T027 [P] [US1] Create AmplifierControlPageLayout struct in src/hardware/display.rs
- [x] T028 [P] [US1] Implement render_power_button method in src/hardware/display.rs
- [x] T029 [US1] Implement render_amplifier_control_page method in src/hardware/display.rs (depends on T027, T028)
- [x] T030 [US1] Add HardwareCommand::UpdateAmplifierPage variant in src/hardware/events.rs
- [x] T031 [US1] Implement power button handling in device_display_loop in src/hardware/device.rs
- [x] T032 [US1] Add power toggle button mapping in main event loop in src/main.rs
- [x] T033 [US1] Implement HomeAssistantCommand::TogglePower handling in MqttClient (sends power toggle to Home Assistant entity) in src/homeassistant/client.rs
- [x] T034 [US1] Implement power state subscription in MqttClient::subscribe_to_entities in src/homeassistant/client.rs
- [x] T035 [US1] Implement power state change event emission in src/homeassistant/client.rs
- [x] T036 [US1] Add validation for power toggle commands in src/controller/state.rs
- [x] T037 [US1] Add connection status indicator rendering in src/hardware/display.rs
- [x] T038 [US1] Spawn MQTT client task in main.rs event loop
- [x] T039 [US1] Route HomeAssistantEvent::PowerStateChanged to ApplicationState in src/main.rs
- [x] T040 [US1] Add error display for failed power commands in src/hardware/display.rs

**Checkpoint**: At this point, User Story 1 should be fully functional - users can view power state and toggle amplifier power from hardware controller

---

## Phase 4: User Story 2 - Select Amplifier Input Sources (Priority: P2)

**Goal**: Enable users to view available input sources and switch between them using hardware controller. Controller tracks selected source locally and sends IR commands via MQTT.

**Independent Test**: Display available amplifier sources on controller, select different sources via buttons, verify IR commands are sent via MQTT and display updates immediately

### Implementation for User Story 2

- [ ] T041 [P] [US2] Extend AmplifierControlPageLayout with source selection fields in src/hardware/display.rs
- [ ] T042 [P] [US2] Implement render_source_button method (display Phono, CD, Spotify, Source4, Source5) in src/hardware/display.rs
- [ ] T043 [US2] Update render_amplifier_control_page to include source display in src/hardware/display.rs (depends on T041, T042)
- [ ] T044 [US2] Create PageView::AmplifierSourceSelection sub-page in src/controller/state.rs
- [ ] T045 [US2] Implement source selection button mapping in main event loop in src/main.rs
- [ ] T046 [US2] Implement HomeAssistantCommand::SelectSource handling in MqttClient (builds IR command from ir_codes.rs and publishes to IR Blaster topic) in src/homeassistant/client.rs
- [ ] T047 [US2] Update local AmplifierState.selected_source when user selects a source in src/controller/state.rs
- [ ] T048 [US2] Persist selected_source to file when changed in src/controller/state.rs
- [ ] T049 [US2] Restore selected_source from file on application startup in src/controller/state.rs
- [ ] T050 [US2] Add validation for source selection commands in src/controller/state.rs
- [ ] T051 [US2] Implement source selection page navigation in src/main.rs
- [ ] T052 [US2] Add error handling for invalid source selection in src/controller/state.rs
- [ ] T053 [US2] Add visual feedback when IR command is sent in src/hardware/display.rs

**Checkpoint**: At this point, User Stories 1 AND 2 should both work independently - users can control power and select sources with local state tracking

---

## Phase 5: User Story 3 - Adjust Amplifier Volume (Priority: P2)

**Goal**: Enable users to adjust amplifier volume using rotary knob. Controller sends IR commands for volume up/down via MQTT.

**Independent Test**: Rotate volume knob on controller, verify IR commands are sent via MQTT with correct timing

### Implementation for User Story 3

- [ ] T054 [P] [US3] Add HardwareEvent::VolumeKnobRotated variant in src/hardware/events.rs (if not already exists)
- [ ] T055 [US3] Implement volume knob event handling in main event loop in src/main.rs
- [ ] T056 [US3] Implement HomeAssistantCommand::VolumeUp handling in MqttClient (builds IR command with 0xC10 and publishes to IR Blaster topic) in src/homeassistant/client.rs
- [ ] T057 [US3] Implement HomeAssistantCommand::VolumeDown handling in MqttClient (builds IR command with 0xC11 and publishes to IR Blaster topic) in src/homeassistant/client.rs
- [ ] T058 [US3] Add rate limiting for volume commands (prevent flooding MQTT with too many commands) in src/homeassistant/client.rs
- [ ] T059 [US3] Add visual feedback for volume adjustment on display in src/hardware/display.rs
- [ ] T060 [US3] Handle rapid knob rotation with proper command queuing in src/main.rs

**Checkpoint**: At this point, User Stories 1, 2, AND 3 should all work independently - users can control power, select sources, and adjust volume

---

## Phase 6: User Story 4 - Navigate Between Snapcast and Amplifier Control Pages (Priority: P3)

**Goal**: Enable seamless navigation between existing Snapcast control pages and new amplifier control page using page buttons

**Independent Test**: Use page buttons to navigate between Snapcast Status, Stream Selection, and Amplifier Control pages, verify smooth transitions and correct display updates

### Implementation for User Story 4

- [ ] T061 [P] [US4] Update page button mapping to include AmplifierControl in src/main.rs
- [ ] T062 [US4] Implement page cycling logic for HardwareEvent::PageButtonPressed in src/main.rs
- [ ] T063 [US4] Add ApplicationState::next_page method in src/controller/state.rs
- [ ] T064 [US4] Add ApplicationState::previous_page method in src/controller/state.rs
- [ ] T065 [US4] Ensure display refreshes on page change in src/main.rs
- [ ] T066 [US4] Add page indicator to all page layouts in src/hardware/display.rs
- [ ] T067 [US4] Update device_display_loop to handle all page types in src/hardware/device.rs
- [ ] T068 [US4] Test concurrent state updates across pages in src/main.rs

**Checkpoint**: All user stories should now be independently functional - users can navigate seamlessly between Snapcast and amplifier controls

---

## Phase 7: Polish & Cross-Cutting Concerns

**Purpose**: Improvements that affect multiple user stories

- [ ] T069 [P] Add command queuing for offline MQTT connection in src/homeassistant/client.rs
- [ ] T070 [P] Implement command timeout detection (5 second threshold) in src/homeassistant/client.rs
- [ ] T071 [P] Add connection health monitoring with keep-alive in src/homeassistant/client.rs
- [ ] T072 [P] Add logging for all MQTT events with tracing/log crate in src/homeassistant/client.rs
- [ ] T073 [P] Optimize display refresh rate to avoid unnecessary renders in src/hardware/device.rs
- [ ] T074 [P] Add graceful shutdown for MQTT client in src/homeassistant/client.rs
- [ ] T075 [P] Validate config file format on startup (check ir_blaster_topic is set) in src/config/settings.rs
- [ ] T076 Code cleanup and remove debug print statements across all files
- [ ] T077 Run cargo clippy and fix all warnings
- [ ] T078 Update CLAUDE.md with Home Assistant integration patterns (IR Blaster, local state)
- [ ] T079 Validate quickstart.md steps match actual implementation

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies - can start immediately
- **Foundational (Phase 2)**: Depends on Setup completion - BLOCKS all user stories
- **User Stories (Phase 3-6)**: All depend on Foundational phase completion
  - User Story 1 (P1): Can start after Phase 2 - Power control with Home Assistant
  - User Story 2 (P2): Can start after Phase 2 - Source selection with local state and IR commands
  - User Story 3 (P2): Can start after Phase 2 - Volume control with IR commands
  - User Story 4 (P3): Can start after Phase 2 - Page navigation integration
- **Polish (Phase 7)**: Depends on all desired user stories being complete

### User Story Dependencies

- **User Story 1 (P1)**: Can start after Foundational (Phase 2) - No dependencies on other stories
- **User Story 2 (P2)**: Can start after Foundational (Phase 2) - Uses IR command infrastructure but independently testable
- **User Story 3 (P2)**: Can start after Foundational (Phase 2) - Uses IR command infrastructure but independently testable
- **User Story 4 (P3)**: Can start after Foundational (Phase 2) - Integrates with existing page views but independently testable

### Within Each User Story

- Display rendering before button/knob event handling
- MQTT client implementation before IR command publishing
- IR command builders before command handling
- State validation before command execution
- Core implementation before error handling
- Story complete before moving to next priority

### Parallel Opportunities

- All Setup tasks marked [P] can run in parallel
- All Foundational tasks marked [P] can run in parallel (within Phase 2)
- Once Foundational phase completes, User Stories 1, 2, 3, and 4 can start in parallel (if team capacity allows)
- Within User Story 1: T027, T028 can run in parallel
- Within User Story 2: T041, T042 can run in parallel
- Within User Story 3: T054, T056, T057 can run in parallel
- All Polish tasks marked [P] can run in parallel

---

## Parallel Example: User Story 1

```bash
# Launch display components together:
Task: "Create AmplifierControlPageLayout struct in src/hardware/display.rs"
Task: "Implement render_power_button method in src/hardware/display.rs"

# After display foundation is ready, launch event handling components:
Task: "Add HardwareCommand::UpdateAmplifierPage variant in src/hardware/events.rs"
Task: "Implement power button handling in device_display_loop in src/hardware/device.rs"
```

---

## Parallel Example: User Story 2

```bash
# Launch source display components together:
Task: "Extend AmplifierControlPageLayout with source selection fields in src/hardware/display.rs"
Task: "Implement render_source_button method in src/hardware/display.rs"

# Launch source IR command handling:
Task: "Implement HomeAssistantCommand::SelectSource handling in MqttClient (IR command builder + MQTT publish)"
Task: "Update local AmplifierState.selected_source when user selects a source"
```

---

## Parallel Example: User Story 3

```bash
# Launch volume command components together:
Task: "Implement HomeAssistantCommand::VolumeUp handling in MqttClient"
Task: "Implement HomeAssistantCommand::VolumeDown handling in MqttClient"

# Launch volume UI feedback:
Task: "Add HardwareEvent::VolumeKnobRotated variant in src/hardware/events.rs"
Task: "Add visual feedback for volume adjustment on display"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup (T001-T003)
2. Complete Phase 2: Foundational (T004-T026) - CRITICAL: blocks all stories
3. Complete Phase 3: User Story 1 (T027-T040)
4. **STOP and VALIDATE**: Test User Story 1 independently
   - Connect to Home Assistant MQTT broker
   - View amplifier power state on hardware display
   - Toggle power via button
   - Verify display updates within 2 seconds
   - Verify power toggle command is sent to Home Assistant
5. Commit and tag as MVP release

### Incremental Delivery

1. Complete Setup + Foundational → Foundation ready (26 tasks)
2. Add User Story 1 → Test independently → Commit (14 tasks) **← MVP!**
3. Add User Story 2 → Test independently → Commit (13 tasks) - Source selection with local state and IR commands
4. Add User Story 3 → Test independently → Commit (7 tasks) - Volume control with IR commands
5. Add User Story 4 → Test independently → Commit (8 tasks) - Page navigation
6. Add Polish → Final release (11 tasks)

Total tasks: **79 tasks**

Each story adds value without breaking previous stories.

### Parallel Team Strategy

With multiple developers:

1. Team completes Setup + Foundational together (T001-T026)
2. Once Foundational is done:
   - Developer A: User Story 1 (T027-T040) - Power control
   - Developer B: User Story 2 (T041-T053) - Source selection with IR Blaster
   - Developer C: User Story 3 (T054-T060) - Volume control with IR Blaster
   - Developer D: User Story 4 (T061-T068) - Page navigation
3. Stories complete and integrate independently

---

## Configuration Example

After implementation, users will add to `config.toml`:

```toml
[homeassistant]
broker_address = "192.168.1.100"
broker_port = 1883
username = "controller"      # optional
password = "secret"          # optional
ir_blaster_topic = "tasmota_17DD9F/cmnd/irsend"  # Configurable IR Blaster MQTT topic

[homeassistant.amplifier]
power_entity = "switch.amplifier_power"  # Home Assistant entity for amplifier power state
```

**Notes:**
- `ir_blaster_topic`: MQTT topic for sending IR commands to the Tasmota IR Blaster device
- Power state is synchronized from Home Assistant (via `power_entity`)
- Source selection state is tracked locally in the application and persisted to a file
- Volume level is not tracked (IR commands are send-only)

---

## Validation Checklist

Before marking this feature complete:

- [x] Power state displays on hardware controller within 3 seconds
- [ ] Power toggle command sends IR command via MQTT within 200ms
- [ ] Power state updates reflect within 2 seconds when changed in Home Assistant
- [ ] Source selection updates display immediately and sends IR command via MQTT within 200ms
- [ ] Volume knob rotation sends IR commands via MQTT within 100ms per step
- [ ] 95% of IR commands successfully published to MQTT when broker is reachable
- [ ] Selected source state persists across application restarts and restores within 1 second
- [ ] Connection failures to MQTT broker detected and displayed within 5 seconds
- [ ] Page navigation completes within 1 second
- [ ] MQTT connection maintains 99% uptime during normal operation
- [ ] All cargo clippy warnings resolved
- [ ] Quickstart.md validation passes
- [ ] IR Blaster topic is configurable in config.toml
- [ ] All 5 sources (Phono, CD, Spotify, Source 4, Source 5) work correctly
- [ ] Volume up/down IR commands use correct codes (0xC10, 0xC11)

---

## Notes

- [P] tasks = different files, no dependencies
- [Story] label maps task to specific user story for traceability
- Each user story should be independently completable and testable
- Constitution principle: Speed Over Perfection - no tests unless explicitly requested
- Constitution principle: Incremental Delivery - each user story is a shippable increment
- Constitution principle: Simplicity First - extend existing patterns, avoid abstractions
- Commit after each task or logical group
- Stop at any checkpoint to validate story independently
- Avoid: vague tasks, same file conflicts, cross-story dependencies that break independence

### Architecture Notes

- **IR Blaster Integration**: Source selection and volume control use one-way IR commands sent via MQTT to a Tasmota IR Blaster device
- **Local State Management**: Source selection state is tracked locally in the application and persisted to a file (not synchronized from Home Assistant)
- **Power State Sync**: Only power state is synchronized from Home Assistant via MQTT (bidirectional)
- **Volume Control**: Volume adjustments are send-only IR commands (no volume level tracking)
- **IR Command Format**: RC5 protocol, 12 bits, specific data codes for each function
- **Configurable Topic**: IR Blaster MQTT topic is configurable in settings to support different Tasmota devices
