# Tasks: Unified Source View

**Input**: Design documents from `/specs/003-unified-source-view/`
**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/, quickstart.md

**Tests**: Per project constitution (Principle I: Speed Over Perfection), comprehensive tests are OPTIONAL and NOT required for prototyping phase. Tasks below focus on implementation without test tasks.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `- [ ] [ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3, US4)
- Include exact file paths in descriptions

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Create new unified source abstraction module and configuration support

- [ ] T001 Create src/models/ directory for unified source data model
- [ ] T002 [P] Add snapcast_amplifier_source configuration field to src/config/settings.rs
- [ ] T003 [P] Create src/models/mod.rs to declare unified_source module
- [ ] T004 Update src/lib.rs or src/main.rs to include models module in crate hierarchy

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core unified source abstraction that ALL user stories depend on

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

- [ ] T005 Define UnifiedSource enum with SnapcastStream and AmplifierInput variants in src/models/unified_source.rs
- [ ] T006 [P] Implement UnifiedSource methods (display_name, is_snapcast_source, identifier, Display trait) in src/models/unified_source.rs
- [ ] T007 [P] Implement Serialize/Deserialize for UnifiedSource using serde with tagged enum format in src/models/unified_source.rs
- [ ] T008 Define UnifiedSourceCollection struct with sources, active_source, snapcast_amplifier_source, selected_index fields in src/models/unified_source.rs
- [ ] T009 Implement UnifiedSourceCollection::new() to build source list from Snapcast streams and amplifier inputs in src/models/unified_source.rs
- [ ] T010 [P] Implement UnifiedSourceCollection::update_snapcast_streams() for dynamic stream updates in src/models/unified_source.rs
- [ ] T011 [P] Implement UnifiedSourceCollection navigation methods (select_previous, select_next, selected_source, confirm_selection) in src/models/unified_source.rs
- [ ] T012 [P] Implement UnifiedSourceCollection state methods (set_active, active_source, all_sources) in src/models/unified_source.rs
- [ ] T013 Add unified_sources field to ApplicationState struct in src/controller/state.rs
- [ ] T014 Initialize unified_sources field in ApplicationState::new() using config snapcast_amplifier_source in src/controller/state.rs
- [ ] T015 Add AmplifierSource::all() method to return all amplifier source variants in src/homeassistant/types.rs

**Checkpoint**: Foundation ready - unified source abstraction is complete and integrated into ApplicationState

---

## Phase 3: User Story 1 - View All Available Sources in Single List (Priority: P1) 🎯 MVP

**Goal**: Users can see all Snapcast streams and non-Snapcast amplifier inputs in a single unified list on the controller display

**Independent Test**: Display all available sources (e.g., "Living Room Stream", "Bedroom Stream", "Phono", "CD", "Spotify") in a single unified list on the hardware controller and verify all sources appear regardless of type

### Implementation for User Story 1

- [ ] T016 [US1] Define PageView::UnifiedSourceSelection variant in hardware display page enum (location based on existing PageView enum)
- [ ] T017 [US1] Implement render_unified_source_view() display function in src/hardware/display.rs
- [ ] T018 [US1] Add unified source list rendering logic with source highlighting in src/hardware/display.rs
- [ ] T019 [US1] Update page navigation cycle to include UnifiedSourceSelection page in src/controller/state.rs
- [ ] T020 [US1] Add handle_snapcast_event() logic to call update_snapcast_streams() on ServerReconnected event in src/controller/state.rs
- [ ] T021 [US1] Wire up UnifiedSourceSelection page rendering in main display loop in src/hardware/display.rs or src/main.rs

**Checkpoint**: At this point, User Story 1 should be fully functional - unified source list displays all Snapcast and amplifier sources

---

## Phase 4: User Story 2 - Switch Between Any Source with Single Action (Priority: P1) 🎯 MVP

**Goal**: Users can switch from any source to any other source with a single button press, with automatic amplifier input switching and stream selection

**Independent Test**: Select different sources from the unified list and verify: (1) selecting a Snapcast stream automatically switches amplifier to Snapcast input and selects stream, (2) selecting a non-Snapcast input switches amplifier to that input

### Implementation for User Story 2

- [ ] T022 [P] [US2] Define SourceActivationContext struct with target_source, clients, and configuration fields in src/models/unified_source.rs
- [ ] T023 [P] [US2] Define ActivationResult enum with SnapcastStreamActivated and AmplifierInputActivated variants in src/models/unified_source.rs
- [ ] T024 [US2] Implement SourceActivationContext::new() constructor in src/models/unified_source.rs
- [ ] T025 [US2] Implement SourceActivationContext::activate() with multi-step logic for Snapcast streams in src/models/unified_source.rs
- [ ] T026 [US2] Implement SourceActivationContext::activate() with single-step logic for amplifier inputs in src/models/unified_source.rs
- [ ] T027 [US2] Add select_unified_source() method to ApplicationState in src/controller/state.rs
- [ ] T028 [US2] Implement source activation orchestration in select_unified_source() (create context, call activate, update state) in src/controller/state.rs
- [ ] T029 [US2] Add unified source selection button handler in src/controller/mapping.rs
- [ ] T030 [US2] Wire up button press to ApplicationState::select_unified_source() in event handler in src/controller/mapping.rs or src/main.rs
- [ ] T031 [US2] Add logging for source activation success/failure in src/controller/state.rs

**Checkpoint**: At this point, User Stories 1 AND 2 should both work - users can view and select sources with automatic activation handling

---

## Phase 5: User Story 3 - Persist Unified Source Selection Across Restarts (Priority: P2)

**Goal**: System remembers which source was active when the controller application restarts and restores that state accurately

**Independent Test**: Select various sources (both Snapcast and non-Snapcast), restart the controller application, and verify the display shows the correct active source after restart

### Implementation for User Story 3

- [ ] T032 [P] [US3] Define UnifiedSourceState struct with active_source and last_updated fields in src/models/unified_source.rs
- [ ] T033 [P] [US3] Implement SystemTime serde serialization module (systemtime_serde) in src/models/unified_source.rs
- [ ] T034 [P] [US3] Implement Serialize/Deserialize for UnifiedSourceState in src/models/unified_source.rs
- [ ] T035 [US3] Add save_unified_source_state() method to ApplicationState in src/controller/state.rs
- [ ] T036 [US3] Implement JSON file persistence with atomic write pattern in save_unified_source_state() in src/controller/state.rs
- [ ] T037 [US3] Add load_unified_source_state() method to ApplicationState in src/controller/state.rs
- [ ] T038 [US3] Implement state restoration from ~/.config/snapcast-controller/unified_source_state.json in src/controller/state.rs
- [ ] T039 [US3] Call save_unified_source_state() after source activation in select_unified_source() in src/controller/state.rs
- [ ] T040 [US3] Call load_unified_source_state() during ApplicationState initialization in src/controller/state.rs or src/main.rs
- [ ] T041 [US3] Add error handling for corrupted/missing state file with graceful fallback in src/controller/state.rs

**Checkpoint**: All core user stories (1, 2, 3) should now be functional - source selection persists across restarts

---

## Phase 6: User Story 4 - Navigate Unified Source View with Existing Controls (Priority: P2)

**Goal**: Users can navigate and select from the unified source list using existing hardware controller buttons and knobs with intuitive interactions

**Independent Test**: Use hardware controller's navigation buttons to scroll through the unified source list, select sources, and verify the interface responds consistently with other controller pages

### Implementation for User Story 4

- [ ] T042 [P] [US4] Add unified_source_select_previous() method to ApplicationState in src/controller/state.rs
- [ ] T043 [P] [US4] Add unified_source_select_next() method to ApplicationState in src/controller/state.rs
- [ ] T044 [P] [US4] Add unified_source_selected() getter method to ApplicationState in src/controller/state.rs
- [ ] T045 [US4] Map hardware navigation up/down buttons to unified source navigation in src/controller/mapping.rs
- [ ] T046 [US4] Map rotary encoder events to unified source navigation in src/controller/mapping.rs
- [ ] T047 [US4] Map selection/confirm button to source activation in src/controller/mapping.rs
- [ ] T048 [US4] Implement list scrolling logic for sources exceeding display capacity in src/hardware/display.rs
- [ ] T049 [US4] Add visual feedback for selection highlight movement in src/hardware/display.rs
- [ ] T050 [US4] Update page navigation button handler to cycle to/from UnifiedSourceSelection page in src/controller/mapping.rs

**Checkpoint**: All user stories (1, 2, 3, 4) are complete - full unified source view feature is functional

---

## Phase 7: Polish & Cross-Cutting Concerns

**Purpose**: Remove deprecated pages and improve user experience across the feature

- [ ] T051 Remove StreamSelection page from PageView enum and navigation cycle
- [ ] T052 Remove SourceSelection page from PageView enum and navigation cycle
- [ ] T053 [P] Remove render_stream_selection() function from src/hardware/display.rs
- [ ] T054 [P] Remove render_source_selection() function from src/hardware/display.rs
- [ ] T055 Add tracing/logging for unified source state synchronization events in src/controller/state.rs
- [ ] T056 [P] Update config.toml.example with snapcast_amplifier_source field documentation
- [ ] T057 [P] Add comments documenting UnifiedSource enum variants and usage in src/models/unified_source.rs
- [ ] T058 Verify unified source view responds within 500ms navigation requirement
- [ ] T059 Verify source activation completes within 2-second requirement
- [ ] T060 Run quickstart.md manual testing checklist validation

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies - can start immediately
- **Foundational (Phase 2)**: Depends on Setup (Phase 1) completion - BLOCKS all user stories
- **User Story 1 (Phase 3)**: Depends on Foundational (Phase 2) completion
- **User Story 2 (Phase 4)**: Depends on User Story 1 (Phase 3) completion - builds on unified view display
- **User Story 3 (Phase 5)**: Depends on Foundational (Phase 2) completion - can start after foundation, independent of US1/US2
- **User Story 4 (Phase 6)**: Depends on User Story 1 (Phase 3) completion - requires unified view to exist
- **Polish (Phase 7)**: Depends on all user stories being complete

### User Story Dependencies

- **User Story 1 (P1)**: Depends on Foundational - displays unified source list
- **User Story 2 (P1)**: Depends on User Story 1 - adds source activation to the display
- **User Story 3 (P2)**: Depends on Foundational - persistence layer is independent, but needs US2 for full value
- **User Story 4 (P2)**: Depends on User Story 1 - navigation requires view to exist

### Within Each User Story

- Foundation tasks (T005-T015) must complete before ANY user story
- User Story 1 provides display foundation for User Story 2 and 4
- User Story 2 provides activation mechanism that User Story 3 persists
- Models/enums before services/logic
- State management before UI integration
- Core implementation before button mapping

### Parallel Opportunities

**Phase 1 (Setup)**: Tasks T002, T003 can run in parallel
**Phase 2 (Foundational)**: Tasks T006-T007, T010-T012, can run in parallel after T005, T008 complete
**Phase 3 (US1)**: No parallel opportunities (display rendering is sequential)
**Phase 4 (US2)**: Tasks T022-T024 can run in parallel; T025-T026 can run in parallel after T024
**Phase 5 (US3)**: Tasks T032-T034 can run in parallel
**Phase 6 (US4)**: Tasks T042-T044 can run in parallel
**Phase 7 (Polish)**: Tasks T053-T054, T056-T057 can run in parallel

---

## Parallel Example: Foundational Phase

```bash
# After T005 and T008 complete, launch these together:
Task: "Implement UnifiedSource methods in src/models/unified_source.rs" (T006)
Task: "Implement Serialize/Deserialize for UnifiedSource in src/models/unified_source.rs" (T007)

# After T009 completes, launch these together:
Task: "Implement update_snapcast_streams() in src/models/unified_source.rs" (T010)
Task: "Implement navigation methods in src/models/unified_source.rs" (T011)
Task: "Implement state methods in src/models/unified_source.rs" (T012)
```

---

## Parallel Example: User Story 3

```bash
# Launch all model definition tasks for persistence together:
Task: "Define UnifiedSourceState struct in src/models/unified_source.rs" (T032)
Task: "Implement SystemTime serde module in src/models/unified_source.rs" (T033)
Task: "Implement Serialize/Deserialize for UnifiedSourceState in src/models/unified_source.rs" (T034)
```

---

## Implementation Strategy

### MVP First (User Stories 1 + 2 Only)

1. Complete Phase 1: Setup
2. Complete Phase 2: Foundational (CRITICAL - blocks all stories)
3. Complete Phase 3: User Story 1 (view sources)
4. Complete Phase 4: User Story 2 (select sources)
5. **STOP and VALIDATE**: Test that users can view and select sources
6. Deploy/demo MVP

**Rationale**: User Stories 1 and 2 are both P1 priority and together deliver the core value proposition: unified source view with single-action switching. User Stories 3 and 4 (both P2) add polish but aren't required for basic functionality.

### Incremental Delivery

1. Complete Setup + Foundational → Foundation ready
2. Add User Story 1 → Test independently → Users can VIEW unified sources
3. Add User Story 2 → Test independently → Users can SELECT sources (MVP! 🎯)
4. Add User Story 3 → Test independently → Selection persists across restarts
5. Add User Story 4 → Test independently → Navigation is polished and intuitive
6. Add Polish phase → Clean up old pages, optimize performance

### Parallel Team Strategy

With multiple developers:

1. Team completes Setup + Foundational together
2. Once Foundational is done:
   - Developer A: User Story 1 (display)
3. After User Story 1 completes:
   - Developer A: User Story 2 (activation)
   - Developer B: User Story 3 (persistence) - can start immediately after foundational
4. After User Story 1 completes:
   - Developer C: User Story 4 (navigation)

**Note**: User Story 3 (persistence) can be developed in parallel with US1/US2 since it only depends on the foundational unified source abstraction, not the UI components.

---

## Task Summary

- **Total Tasks**: 60
- **Setup Phase**: 4 tasks
- **Foundational Phase**: 11 tasks (BLOCKS all user stories)
- **User Story 1 (P1)**: 6 tasks
- **User Story 2 (P1)**: 10 tasks
- **User Story 3 (P2)**: 10 tasks
- **User Story 4 (P2)**: 9 tasks
- **Polish Phase**: 10 tasks

### Tasks per User Story Breakdown

- **US1 (View sources)**: 6 implementation tasks
- **US2 (Select sources)**: 10 implementation tasks
- **US3 (Persistence)**: 10 implementation tasks
- **US4 (Navigation)**: 9 implementation tasks

### Parallel Opportunities Identified

- Phase 1: 2 parallel tasks
- Phase 2: 6 parallel tasks (in 2 batches)
- Phase 4: 4 parallel tasks (in 2 batches)
- Phase 5: 3 parallel tasks
- Phase 6: 3 parallel tasks
- Phase 7: 4 parallel tasks

**Total parallel opportunities**: 22 tasks can be executed concurrently (37% of all tasks)

### MVP Scope (Suggested)

**Recommended MVP**: Complete through Phase 4 (User Stories 1 + 2)
- Total MVP tasks: 31 tasks (Setup + Foundational + US1 + US2)
- Delivers core value: unified view with single-action source switching
- Can be extended incrementally with US3 (persistence) and US4 (navigation polish)

---

## Notes

- [P] tasks = different files, no dependencies on incomplete work
- [Story] label maps task to specific user story for traceability
- Each user story should be independently testable (see Independent Test criteria in each phase)
- No comprehensive tests per constitution Principle I (Speed Over Perfection)
- Manual testing checklist available in quickstart.md
- Commit after each task or logical group
- Stop at any checkpoint to validate story independently
- File paths assume single Rust project structure per plan.md
