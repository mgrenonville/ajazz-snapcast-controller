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

- [ ] T001 Add rumqttc dependency to Cargo.toml (version 0.24)
- [ ] T002 [P] Create src/homeassistant/ module directory
- [ ] T003 [P] Add homeassistant module declaration to src/main.rs

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core infrastructure that MUST be complete before ANY user story can be implemented

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

- [ ] T004 Create HomeAssistantConfig struct in src/config/settings.rs
- [ ] T005 [P] Create AmplifierEntityConfig struct in src/config/settings.rs
- [ ] T006 Extend ConnectionSettings with homeassistant field in src/config/settings.rs
- [ ] T007 [P] Create EntityAvailability enum in src/homeassistant/types.rs
- [ ] T008 [P] Create AmplifierState struct in src/homeassistant/types.rs
- [ ] T009 [P] Create HomeAssistantConnection struct in src/homeassistant/types.rs
- [ ] T010 [P] Create HomeAssistantEvent enum in src/homeassistant/types.rs
- [ ] T011 [P] Create HomeAssistantCommand enum in src/homeassistant/types.rs
- [ ] T012 Create homeassistant module exports in src/homeassistant/mod.rs
- [ ] T013 Create MqttClient struct skeleton in src/homeassistant/client.rs
- [ ] T014 Implement MQTT connection logic in src/homeassistant/client.rs
- [ ] T015 Implement topic subscription logic in src/homeassistant/client.rs
- [ ] T016 Implement command publishing logic in src/homeassistant/client.rs
- [ ] T017 Implement error handling and reconnection with exponential backoff in src/homeassistant/client.rs
- [ ] T018 Extend ApplicationState with homeassistant_connected field in src/controller/state.rs
- [ ] T019 Extend ApplicationState with amplifier field in src/controller/state.rs
- [ ] T020 Add PageView::AmplifierControl variant to src/controller/state.rs
- [ ] T021 Implement ApplicationState::handle_homeassistant_event method in src/controller/state.rs

**Checkpoint**: Foundation ready - user story implementation can now begin in parallel

---

## Phase 3: User Story 1 - Monitor and Control Amplifier Power (Priority: P1) 🎯 MVP

**Goal**: Enable users to view amplifier power state on hardware display and toggle power on/off via controller button

**Independent Test**: Connect controller to Home Assistant, view amplifier power state on display, toggle power via button, verify display updates and physical amplifier responds within 2 seconds

### Implementation for User Story 1

- [ ] T022 [P] [US1] Create AmplifierControlPageLayout struct in src/hardware/display.rs
- [ ] T023 [P] [US1] Implement render_power_button method in src/hardware/display.rs
- [ ] T024 [US1] Implement render_amplifier_control_page method in src/hardware/display.rs (depends on T022, T023)
- [ ] T025 [US1] Add HardwareCommand::UpdateAmplifierPage variant in src/hardware/events.rs
- [ ] T026 [US1] Implement power button handling in device_display_loop in src/hardware/device.rs
- [ ] T027 [US1] Add power toggle button mapping in main event loop in src/main.rs
- [ ] T028 [US1] Implement HomeAssistantCommand::TogglePower handling in MqttClient in src/homeassistant/client.rs
- [ ] T029 [US1] Implement power state subscription in MqttClient::subscribe_to_entities in src/homeassistant/client.rs
- [ ] T030 [US1] Implement power state change event emission in src/homeassistant/client.rs
- [ ] T031 [US1] Add validation for power toggle commands in src/controller/state.rs
- [ ] T032 [US1] Add connection status indicator rendering in src/hardware/display.rs
- [ ] T033 [US1] Spawn MQTT client task in main.rs event loop
- [ ] T034 [US1] Route HomeAssistantEvent::EntityStateChanged to ApplicationState in src/main.rs
- [ ] T035 [US1] Add error display for failed power commands in src/hardware/display.rs

**Checkpoint**: At this point, User Story 1 should be fully functional - users can view power state and toggle amplifier power from hardware controller

---

## Phase 4: User Story 2 - Select Amplifier Input Sources (Priority: P2)

**Goal**: Enable users to view available input sources and switch between them using hardware controller

**Independent Test**: Display available amplifier sources on controller, select different sources via buttons, verify amplifier switches input and display reflects current source within 2 seconds

### Implementation for User Story 2

- [ ] T036 [P] [US2] Extend AmplifierControlPageLayout with source selection fields in src/hardware/display.rs
- [ ] T037 [P] [US2] Implement render_source_button method in src/hardware/display.rs
- [ ] T038 [US2] Update render_amplifier_control_page to include source display in src/hardware/display.rs (depends on T036, T037)
- [ ] T039 [US2] Create PageView::AmplifierSourceSelection sub-page in src/controller/state.rs
- [ ] T040 [US2] Implement source selection button mapping in main event loop in src/main.rs
- [ ] T041 [US2] Implement HomeAssistantCommand::SelectSource handling in MqttClient in src/homeassistant/client.rs
- [ ] T042 [US2] Implement source state subscription in MqttClient::subscribe_to_entities in src/homeassistant/client.rs
- [ ] T043 [US2] Implement source state change event emission in src/homeassistant/client.rs
- [ ] T044 [US2] Add validation for source selection commands in src/controller/state.rs
- [ ] T045 [US2] Parse available_sources from MQTT discovery or state topic in src/homeassistant/client.rs
- [ ] T046 [US2] Implement source selection page navigation in src/main.rs
- [ ] T047 [US2] Add error handling for invalid source selection in src/controller/state.rs
- [ ] T048 [US2] Implement amplifier off check before source selection in src/controller/state.rs

**Checkpoint**: At this point, User Stories 1 AND 2 should both work independently - users can control power and select sources

---

## Phase 5: User Story 3 - Navigate Between Snapcast and Amplifier Control Pages (Priority: P3)

**Goal**: Enable seamless navigation between existing Snapcast control pages and new amplifier control page using page buttons

**Independent Test**: Use page buttons to navigate between Snapcast Status, Stream Selection, and Amplifier Control pages, verify smooth transitions and correct display updates

### Implementation for User Story 3

- [ ] T049 [P] [US3] Update page button mapping to include AmplifierControl in src/main.rs
- [ ] T050 [US3] Implement page cycling logic for HardwareEvent::PageButtonPressed in src/main.rs
- [ ] T051 [US3] Add ApplicationState::next_page method in src/controller/state.rs
- [ ] T052 [US3] Add ApplicationState::previous_page method in src/controller/state.rs
- [ ] T053 [US3] Ensure display refreshes on page change in src/main.rs
- [ ] T054 [US3] Add page indicator to all page layouts in src/hardware/display.rs
- [ ] T055 [US3] Update device_display_loop to handle all page types in src/hardware/device.rs
- [ ] T056 [US3] Test concurrent state updates across pages in src/main.rs

**Checkpoint**: All user stories should now be independently functional - users can navigate seamlessly between Snapcast and amplifier controls

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: Improvements that affect multiple user stories

- [ ] T057 [P] Add command queuing for offline MQTT connection in src/homeassistant/client.rs
- [ ] T058 [P] Implement command timeout detection (5 second threshold) in src/homeassistant/client.rs
- [ ] T059 [P] Add connection health monitoring with keep-alive in src/homeassistant/client.rs
- [ ] T060 [P] Add logging for all MQTT events with tracing/log crate in src/homeassistant/client.rs
- [ ] T061 [P] Optimize display refresh rate to avoid unnecessary renders in src/hardware/device.rs
- [ ] T062 [P] Add graceful shutdown for MQTT client in src/homeassistant/client.rs
- [ ] T063 [P] Validate config file format on startup in src/config/settings.rs
- [ ] T064 Code cleanup and remove debug print statements across all files
- [ ] T065 Run cargo clippy and fix all warnings
- [ ] T066 Update CLAUDE.md with Home Assistant integration patterns
- [ ] T067 Validate quickstart.md steps match actual implementation

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies - can start immediately
- **Foundational (Phase 2)**: Depends on Setup completion - BLOCKS all user stories
- **User Stories (Phase 3+)**: All depend on Foundational phase completion
  - User Story 1 (P1): Can start after Phase 2
  - User Story 2 (P2): Can start after Phase 2 (independent of US1, but integrates with amplifier state from US1)
  - User Story 3 (P3): Can start after Phase 2 (requires page structure from US1/US2 but adds navigation layer)
- **Polish (Phase 6)**: Depends on all desired user stories being complete

### User Story Dependencies

- **User Story 1 (P1)**: Can start after Foundational (Phase 2) - No dependencies on other stories
- **User Story 2 (P2)**: Can start after Foundational (Phase 2) - Extends amplifier control page from US1 but independently testable
- **User Story 3 (P3)**: Can start after Foundational (Phase 2) - Integrates with existing page views but independently testable

### Within Each User Story

- Display rendering before button event handling
- MQTT client implementation before command handling
- State validation before command execution
- Core implementation before error handling
- Story complete before moving to next priority

### Parallel Opportunities

- All Setup tasks marked [P] can run in parallel
- All Foundational tasks marked [P] can run in parallel (within Phase 2)
- Once Foundational phase completes, User Stories 1, 2, and 3 can start in parallel (if team capacity allows)
- Within User Story 1: T022, T023 can run in parallel
- Within User Story 2: T036, T037 can run in parallel
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

# Launch source MQTT handling components:
Task: "Implement HomeAssistantCommand::SelectSource handling in MqttClient in src/homeassistant/client.rs"
Task: "Implement source state subscription in MqttClient::subscribe_to_entities in src/homeassistant/client.rs"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup (T001-T003)
2. Complete Phase 2: Foundational (T004-T021) - CRITICAL: blocks all stories
3. Complete Phase 3: User Story 1 (T022-T035)
4. **STOP and VALIDATE**: Test User Story 1 independently
   - Connect to Home Assistant MQTT broker
   - View amplifier power state on hardware display
   - Toggle power via button
   - Verify display updates within 2 seconds
   - Verify physical amplifier responds
5. Commit and tag as MVP release

### Incremental Delivery

1. Complete Setup + Foundational → Foundation ready (21 tasks)
2. Add User Story 1 → Test independently → Commit (14 tasks) **← MVP!**
3. Add User Story 2 → Test independently → Commit (13 tasks)
4. Add User Story 3 → Test independently → Commit (8 tasks)
5. Add Polish → Final release (11 tasks)

Total tasks: **67 tasks**

Each story adds value without breaking previous stories.

### Parallel Team Strategy

With multiple developers:

1. Team completes Setup + Foundational together (T001-T021)
2. Once Foundational is done:
   - Developer A: User Story 1 (T022-T035)
   - Developer B: User Story 2 (T036-T048) - starts after foundational types are in place
   - Developer C: User Story 3 (T049-T056) - starts after page structures exist
3. Stories complete and integrate independently

---

## Configuration Example

After implementation, users will add to `config.toml`:

```toml
[homeassistant]
broker_address = "192.168.1.100"
broker_port = 1883
username = "controller"  # optional
password = "secret"      # optional

[homeassistant.amplifier]
power_entity = "switch.amplifier_power"
source_entity = "input_select.amplifier_source"
```

---

## Validation Checklist

Before marking this feature complete:

- [ ] Power state displays on hardware controller within 3 seconds
- [ ] Power toggle command completes within 3 seconds
- [ ] State updates reflect within 2 seconds
- [ ] Source selection works and updates within 3 seconds
- [ ] 95% command success rate when Home Assistant reachable
- [ ] Connection failures detected and displayed within 5 seconds
- [ ] Page navigation completes within 1 second
- [ ] MQTT connection maintains 99% uptime during normal operation
- [ ] All cargo clippy warnings resolved
- [ ] Quickstart.md validation passes

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
