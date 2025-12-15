# Tasks: Snapcast Controller Application

**Input**: Design documents from `/specs/001-snapcast-controller/`
**Prerequisites**: plan.md (required), spec.md (required for user stories), research.md, data-model.md, contracts/

**Tests**: Per constitution (Speed Over Perfection principle), tests are OPTIONAL and not included in this task list. Focus is on rapid prototyping and working functionality.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

## Path Conventions

- **Single project**: `src/`, `Cargo.toml` at repository root
- Paths shown below use absolute paths from repository root

---

## Phase 1: Setup (Shared Infrastructure) ✅ COMPLETE

**Purpose**: Project initialization and basic structure

- [x] T001 Create Rust project structure with cargo init at repository root
- [x] T002 [P] Create src/hardware/ module directory
- [x] T003 [P] Create src/snapcast/ module directory
- [x] T004 [P] Create src/config/ module directory
- [x] T005 [P] Create src/controller/ module directory
- [x] T006 Add dependencies to Cargo.toml: tokio, serde, toml, anyhow, thiserror
- [x] T007 Add external crate dependencies to Cargo.toml: ajazz_sdk, snapcast_control
- [x] T008 Create config.toml.example in repository root with server/room configuration template

---

## Phase 2: Foundational (Blocking Prerequisites) ✅ COMPLETE

**Purpose**: Core infrastructure that MUST be complete before ANY user story can be implemented

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

- [x] T009 Create ConnectionSettings struct in src/config/settings.rs with serde derives
- [x] T010 Implement TOML config loading in src/config/settings.rs using serde and toml crates
- [x] T011 Implement config validation in src/config/settings.rs (server address, port range, non-empty client_id)
- [x] T012 Create error types module in src/hardware/mod.rs using thiserror (HardwareError enum)
- [x] T013 Create error types module in src/snapcast/mod.rs using thiserror (SnapcastError enum)
- [x] T014 Create RoomState struct in src/snapcast/types.rs with all fields from data-model.md
- [x] T015 [P] Create AudioStream struct in src/snapcast/types.rs with StreamStatus enum
- [x] T016 [P] Create HardwareEvent enum in src/hardware/events.rs (KnobRotated, ButtonPressed, etc.)
- [x] T017 [P] Create SnapcastEvent enum in src/snapcast/types.rs (ClientVolumeChanged, etc.)
- [x] T018 [P] Create ControllerCommand enum in src/controller/mapping.rs (SetVolume, SetMute, AssignStream)
- [x] T019 Create ApplicationState struct in src/controller/state.rs with all fields from data-model.md

**Checkpoint**: Foundation ready - user story implementation can now begin in parallel

---

## Phase 3: User Story 1 - Connect to Room's Audio Client (Priority: P1) 🎯 MVP

**Goal**: Establish connection between hardware controller, Snapcast server, and assign to specific room

**Independent Test**: Connect hardware via USB, configure server/room in config file, run application, verify connection status shown on hardware screens

### Implementation for User Story 1

- [x] T020 [P] [US1] Implement USB HID device detection in src/hardware/device.rs using ajazz_sdk
- [x] T021 [P] [US1] Implement Snapcast server TCP connection in src/snapcast/client.rs using snapcast_control
- [x] T022 [US1] Implement Server.GetStatus JSON-RPC call in src/snapcast/client.rs
- [x] T023 [US1] Parse Server.GetStatus response to extract room state in src/snapcast/client.rs
- [x] T024 [US1] Implement config file loading on application startup in src/main.rs
- [x] T025 [US1] Create hardware device connection handler in src/hardware/device.rs (handle DeviceConnected/Disconnected events)
- [x] T026 [US1] Create Snapcast server connection handler in src/snapcast/client.rs (handle connection/disconnection)
- [x] T027 [US1] Implement basic screen display for connection status in src/hardware/display.rs
- [x] T028 [US1] Display "Waiting for hardware..." message when controller not connected in src/hardware/display.rs
- [x] T029 [US1] Display "Connecting to server..." message during connection attempt in src/hardware/display.rs
- [x] T030 [US1] Display connection error messages on hardware screens in src/hardware/display.rs
- [x] T031 [US1] Display room name and connection success on hardware screens in src/hardware/display.rs
- [x] T032 [US1] Implement main event loop in src/main.rs with tokio::main macro
- [x] T033 [US1] Create tokio task for hardware event listening in src/main.rs
- [x] T034 [US1] Create tokio task for Snapcast network event listening in src/main.rs
- [x] T035 [US1] Initialize ApplicationState with loaded config in src/main.rs
- [x] T036 [US1] Handle HardwareEvent::DeviceConnected in controller in src/controller/state.rs
- [x] T037 [US1] Handle SnapcastEvent::ServerReconnected in controller in src/controller/state.rs

**Checkpoint**: At this point, User Story 1 should be fully functional and testable independently. Can verify USB hardware connection, Snapcast server connection, and room assignment. ✅ COMPLETE

---

## Phase 4: User Story 2 - Monitor Room Audio Status (Priority: P2)

**Goal**: Display real-time room audio status on hardware controller screens

**Independent Test**: With US1 complete, play audio in assigned room, verify volume/mute/stream displayed and updates within 2 seconds

### Implementation for User Story 2

- [x] T038 [P] [US2] Implement JSON-RPC notification parsing in src/snapcast/client.rs
- [x] T039 [P] [US2] Subscribe to Client.OnVolumeChanged notifications in src/snapcast/client.rs
- [x] T040 [P] [US2] Subscribe to Stream.OnUpdate notifications in src/snapcast/client.rs
- [x] T041 [P] [US2] Subscribe to Group.OnStreamChanged notifications in src/snapcast/client.rs
- [x] T042 [US2] Implement screen layout for status page in src/hardware/display.rs (6 button screens)
- [x] T043 [US2] Render volume percentage on button screen 2 in src/hardware/display.rs
- [x] T044 [US2] Render mute status on button screen 0 in src/hardware/display.rs
- [x] T045 [US2] Render current stream name on button screen 1 in src/hardware/display.rs
- [x] T046 [US2] Render connection status on button screen 3 in src/hardware/display.rs
- [x] T047 [US2] Render server address on button screen 4 in src/hardware/display.rs
- [x] T048 [US2] Render room name on button screen 5 in src/hardware/display.rs
- [x] T049 [US2] Handle SnapcastEvent::ClientVolumeChanged in src/controller/state.rs (update RoomState)
- [x] T050 [US2] Handle SnapcastEvent::StreamChanged in src/controller/state.rs (update RoomState.stream_id)
- [x] T051 [US2] Handle Stream.OnUpdate notification in src/controller/state.rs (update AudioStream metadata)
- [x] T052 [US2] Trigger screen refresh when RoomState changes in src/controller/state.rs
- [x] T053 [US2] Implement screen update batching to avoid USB bandwidth saturation in src/hardware/display.rs
- [x] T054 [US2] Add 2-second timeout validation for state change → screen update in src/controller/state.rs

**Checkpoint**: At this point, User Stories 1 AND 2 should both work independently. Can monitor real-time audio status on hardware screens.

---

## Phase 5: User Story 3 - Control Room Audio Playback (Priority: P3)

**Goal**: Control audio playback via hardware knobs and buttons

**Independent Test**: With US1+US2 complete, rotate knobs and press buttons, verify volume/mute/stream commands execute and audio responds

### Implementation for User Story 3

- [x] T055 [P] [US3] Implement knob rotation event handling in src/hardware/events.rs
- [x] T056 [P] [US3] Implement button press/release event handling in src/hardware/events.rs
- [x] T057 [P] [US3] Implement page button handling in src/hardware/events.rs
- [x] T058 [US3] Map knob 0 rotation to volume adjustment in src/controller/mapping.rs (delta * 5% per rotation)
- [x] T059 [US3] Clamp volume to 0-100 range in src/controller/mapping.rs
- [x] T060 [US3] Map button 0 press to mute/unmute toggle in src/controller/mapping.rs
- [x] T061 [US3] Map button 1 press to switch to stream selection page in src/controller/mapping.rs
- [x] T062 [US3] Implement Client.SetVolume JSON-RPC call in src/snapcast/commands.rs
- [x] T063 [US3] Implement Group.SetStream JSON-RPC call in src/snapcast/commands.rs (requires finding group ID first)
- [x] T064 [US3] Find group containing room's client from Server.GetStatus in src/snapcast/client.rs
- [x] T065 [US3] Handle HardwareEvent::KnobRotated in src/controller/state.rs
- [x] T066 [US3] Send ControllerCommand::SetVolume to Snapcast when knob rotated in src/controller/state.rs
- [ ] T067 [US3] Handle HardwareEvent::ButtonPressed for button 0 (mute toggle) in src/controller/state.rs
- [ ] T068 [US3] Send ControllerCommand::SetMute to Snapcast when button 0 pressed in src/controller/state.rs
- [ ] T069 [US3] Implement stream selection page screen layout in src/hardware/display.rs (buttons 0-5 show streams)
- [ ] T070 [US3] Handle page button press to switch between Status and StreamSelection views in src/controller/state.rs
- [ ] T071 [US3] Handle button press in StreamSelection page to select stream in src/controller/state.rs
- [ ] T072 [US3] Send ControllerCommand::AssignStream when stream button pressed in src/controller/state.rs
- [ ] T073 [US3] Implement error display on screens when command fails in src/hardware/display.rs
- [ ] T074 [US3] Add 500ms latency validation for control command → feedback in src/controller/state.rs
- [ ] T075 [US3] Handle rapid successive control inputs (queue or debounce) in src/controller/state.rs

**Checkpoint**: All user stories should now be independently functional. Full audio control via hardware controller working.

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: Improvements that affect multiple user stories

- [ ] T076 [P] Implement exponential backoff reconnection for Snapcast server in src/snapcast/client.rs
- [ ] T077 [P] Implement exponential backoff reconnection for hardware controller in src/hardware/device.rs
- [ ] T078 Handle edge case: room no longer exists on server in src/controller/state.rs
- [ ] T079 Handle edge case: server state changes during command execution in src/controller/state.rs
- [ ] T080 Add logging with log levels (info, warn, error) throughout application
- [ ] T081 Implement graceful shutdown handling (SIGTERM, SIGINT) in src/main.rs
- [ ] T082 Create example config file with comments explaining each field in config.toml.example
- [ ] T083 Add config file creation on first run if not exists in src/config/settings.rs
- [ ] T084 Validate application startup time <3 seconds in src/main.rs

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies - can start immediately
- **Foundational (Phase 2)**: Depends on Setup completion - BLOCKS all user stories
- **User Stories (Phase 3-5)**: All depend on Foundational phase completion
  - User stories can then proceed in parallel (if staffed)
  - Or sequentially in priority order (P1 → P2 → P3)
- **Polish (Phase 6)**: Depends on all desired user stories being complete

### User Story Dependencies

- **User Story 1 (P1)**: Can start after Foundational (Phase 2) - No dependencies on other stories
- **User Story 2 (P2)**: Can start after Foundational (Phase 2) - Builds on US1 conceptually but independently testable
- **User Story 3 (P3)**: Can start after Foundational (Phase 2) - Builds on US1+US2 conceptually but independently testable

**Note**: While US2 and US3 build conceptually on US1, they are technically independent once the foundation is complete. Each can be implemented and tested separately.

### Within Each User Story

- **User Story 1**:
  - T020, T021 can run in parallel (different files)
  - T024-T037 are sequential (depend on initialization)

- **User Story 2**:
  - T038-T041 can run in parallel (notification handling)
  - T042-T048 depend on T042 (screen layout first, then render components)
  - T049-T054 are sequential (state management then screen updates)

- **User Story 3**:
  - T055-T057 can run in parallel (event handling)
  - T062-T064 can run in parallel (JSON-RPC calls)
  - T065-T075 are mostly sequential (command handling flow)

### Parallel Opportunities

- All Setup tasks (T002-T005) can run in parallel
- Foundational types (T014-T018) can run in parallel
- Within US1: Hardware and Snapcast integration (T020, T021) in parallel
- Within US2: Notification subscriptions (T038-T041) in parallel
- Within US3: Event handlers (T055-T057) and JSON-RPC commands (T062-T064) in parallel
- Polish tasks (T076-T077) can run in parallel

---

## Parallel Example: User Story 1

```bash
# Launch hardware and Snapcast integration together:
Task: "T020 [P] [US1] Implement USB HID device detection in src/hardware/device.rs"
Task: "T021 [P] [US1] Implement Snapcast server TCP connection in src/snapcast/client.rs"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup
2. Complete Phase 2: Foundational (CRITICAL - blocks all stories)
3. Complete Phase 3: User Story 1
4. **STOP and VALIDATE**: Test User Story 1 independently
   - Plug in hardware controller
   - Configure server/room in config file
   - Run application
   - Verify connection status on screens
5. Deploy/demo if ready

### Incremental Delivery

1. Complete Setup + Foundational → Foundation ready
2. Add User Story 1 → Test independently → Deploy/Demo (MVP!)
3. Add User Story 2 → Test independently → Deploy/Demo (Can now see real-time status)
4. Add User Story 3 → Test independently → Deploy/Demo (Full control functionality)
5. Add Polish phase → Production-ready deployment
6. Each story adds value without breaking previous stories

### Parallel Team Strategy

With multiple developers:

1. Team completes Setup + Foundational together
2. Once Foundational is done:
   - Developer A: User Story 1
   - Developer B: User Story 2 (can start independently)
   - Developer C: User Story 3 (can start independently)
3. Stories complete and integrate independently

**Note**: While stories can be developed in parallel, there is logical value in completing US1 first to enable end-to-end testing of US2 and US3.

---

## Notes

- [P] tasks = different files, no dependencies
- [Story] label maps task to specific user story for traceability
- Each user story should be independently completable and testable
- Commit after each task or logical group
- Stop at any checkpoint to validate story independently
- Tests are optional per constitution (Speed Over Perfection principle)
- Focus on working functionality first, refine later
