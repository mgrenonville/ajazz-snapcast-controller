# Feature Specification: Unified Source View

**Feature Branch**: `003-unified-source-view`
**Created**: 2026-01-05
**Status**: Draft
**Input**: User description: "Now that each feature is working, I would like to work on user experience. In my point of view, we have 2 different components (snapcast + each sources) and my amplifier. My amplifier has 5 sources and one is snapcast. I would like to show to the device user a unified view of sources (snapcast + amplifier) without having to think that snapcast is in fact a single source on the amplifier. I think we have to introduce a component to encapsulate this."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - View All Available Sources in Single List (Priority: P1)

Users need to see all available audio sources (both Snapcast streams and non-Snapcast amplifier inputs) in a single unified list on the controller, without needing to understand that Snapcast is technically one of the amplifier's inputs.

**Why this priority**: This is the foundational user experience improvement. Users expect a simple list of "what can I listen to" rather than navigating between separate Snapcast and amplifier interfaces. This delivers immediate value by simplifying the mental model required to use the system.

**Independent Test**: Can be fully tested by displaying all available sources (e.g., "Living Room Stream", "Bedroom Stream", "Phono", "CD", "Spotify") in a single unified list on the hardware controller and verifying that all sources appear regardless of whether they're Snapcast streams or direct amplifier inputs.

**Acceptance Scenarios**:

1. **Given** the controller is displaying the unified source view, **When** the user views the source list, **Then** all Snapcast streams and all non-Snapcast amplifier inputs are displayed together in a single list
2. **Given** the unified source list is displayed, **When** the list contains both Snapcast and non-Snapcast sources, **Then** each source is clearly labeled with its name (e.g., stream name or input name) without requiring users to know the underlying technology
3. **Given** the unified source view is active, **When** the currently active source is a Snapcast stream, **Then** that stream is highlighted in the list and the amplifier is set to the Snapcast input
4. **Given** the unified source view is active, **When** the currently active source is a non-Snapcast amplifier input, **Then** that input is highlighted in the list

---

### User Story 2 - Switch Between Any Source with Single Action (Priority: P1)

Users need to switch from any source to any other source (whether Snapcast stream or amplifier input) with a single button press, with the system automatically handling the necessary amplifier input switching and stream selection.

**Why this priority**: This is the core value proposition of the unified view. Users want to select "what to listen to" without understanding or performing multi-step operations. The system should automatically ensure the amplifier is on the correct input when selecting a Snapcast stream, or switch away from Snapcast when selecting another amplifier source.

**Independent Test**: Can be tested by selecting different sources from the unified list and verifying that: (1) when selecting a Snapcast stream, the amplifier automatically switches to the Snapcast input and the correct stream is selected, and (2) when selecting a non-Snapcast input, the amplifier switches to that input.

**Acceptance Scenarios**:

1. **Given** the user is listening to a Snapcast stream, **When** the user selects a non-Snapcast amplifier input (e.g., "Phono"), **Then** the system switches the amplifier to the Phono input and updates the display to show "Phono" as active
2. **Given** the user is listening to a non-Snapcast amplifier input, **When** the user selects a Snapcast stream, **Then** the system switches the amplifier to the Snapcast input and selects the chosen stream on the Snapcast server
3. **Given** the user is listening to one Snapcast stream, **When** the user selects a different Snapcast stream, **Then** the system changes the active stream without changing the amplifier input (which remains on Snapcast)
4. **Given** a source selection is in progress, **When** the necessary commands are sent (amplifier input change and/or stream selection), **Then** the unified view updates to reflect the active source within 2 seconds

---

### User Story 3 - Persist Unified Source Selection Across Restarts (Priority: P2)

Users expect the system to remember which source was active (whether Snapcast stream or amplifier input) when the controller application restarts, and to restore that state accurately.

**Why this priority**: Users expect consistency and continuity. After power cycles or application restarts, the system should resume from the last known state. This builds on the core unified view functionality but is slightly lower priority than the basic selection capability.

**Independent Test**: Can be tested by selecting various sources (both Snapcast and non-Snapcast), restarting the controller application, and verifying that the display shows the correct active source after restart.

**Acceptance Scenarios**:

1. **Given** a Snapcast stream was active before application restart, **When** the application starts, **Then** the unified source view displays that Snapcast stream as the active source
2. **Given** a non-Snapcast amplifier input was active before application restart, **When** the application starts, **Then** the unified source view displays that amplifier input as the active source
3. **Given** the persisted state indicates a Snapcast stream is active but the Snapcast server is unreachable at startup, **When** the application starts, **Then** the unified view indicates the intended source but shows connection status
4. **Given** the persisted state is corrupted or missing, **When** the application starts, **Then** the system defaults to a safe state (e.g., first available source or last known good state)

---

### User Story 4 - Navigate Unified Source View with Existing Controls (Priority: P2)

Users need to navigate and select from the unified source list using the existing hardware controller buttons and knobs, with intuitive interactions that fit the current controller workflow.

**Why this priority**: This ensures the unified source view integrates seamlessly with the existing user interface paradigm. While important for usability, the basic selection mechanism can be tested independently of navigation polish.

**Independent Test**: Can be tested by using the hardware controller's navigation buttons to scroll through the unified source list, select sources, and verify that the interface responds consistently with other controller pages.

**Acceptance Scenarios**:

1. **Given** the unified source view is displayed, **When** the user presses the navigation buttons (up/down or rotary encoder), **Then** the source selection highlight moves to the next/previous source in the list
2. **Given** a source is highlighted in the unified view, **When** the user presses the selection button, **Then** the system activates that source using the appropriate mechanism (stream selection or amplifier input change)
3. **Given** the unified source view has many sources, **When** the user scrolls past the visible area, **Then** the list scrolls to show additional sources
4. **Given** the user is on the unified source view, **When** the user presses the page navigation button, **Then** the controller navigates to other pages (e.g., Snapcast status, amplifier controls) as expected

---

### Edge Cases

- What happens when Snapcast server becomes unreachable while displaying the unified source view that includes Snapcast streams?
- How does the system handle situations where the amplifier input and Snapcast server state are out of sync (e.g., amplifier is on Phono input but system thinks a Snapcast stream is active)?
- What occurs when a Snapcast stream is added or removed while the unified source view is displayed?
- How does the system respond when the amplifier is powered off but the user attempts to select a source?
- What happens when the user rapidly switches between multiple sources?
- How does the system handle the transition period when switching from a non-Snapcast source to a Snapcast stream (amplifier input switch + stream selection)?
- What occurs when the persisted unified source state conflicts with the actual states reported by Snapcast and the amplifier?
- How does the controller indicate loading or transitioning states during source switches?
- What happens if IR commands to switch amplifier input fail while attempting to activate a Snapcast stream?

## Requirements *(mandatory)*

### Functional Requirements

**Unified Source Abstraction:**
- **FR-001**: System MUST provide a unified source abstraction layer that presents both Snapcast streams and non-Snapcast amplifier inputs as equivalent "sources" to the user interface
- **FR-002**: System MUST aggregate available Snapcast streams (retrieved from Snapcast server) and configured amplifier inputs into a single source collection
- **FR-003**: System MUST track which source type each entry represents (Snapcast stream or amplifier input) internally without exposing this distinction to the user

**Source Display:**
- **FR-004**: System MUST display all available sources in a single unified list on the hardware controller
- **FR-005**: System MUST clearly identify each source with a user-friendly name (stream name for Snapcast sources, input name for amplifier sources)
- **FR-006**: System MUST indicate the currently active source in the unified list with visual highlighting
- **FR-007**: System MUST support scrolling or pagination when the number of sources exceeds the display capacity

**Source Selection:**
- **FR-008**: System MUST allow users to select any source from the unified list with a single user action
- **FR-009**: When a Snapcast stream is selected, system MUST automatically switch the amplifier to the Snapcast input (if not already selected) and then select the chosen stream on the Snapcast server
- **FR-010**: When a non-Snapcast amplifier input is selected, system MUST switch the amplifier to that input
- **FR-011**: System MUST handle the multi-step process of amplifier input switching and stream selection transparently, presenting it as a single atomic operation to the user
- **FR-012**: System MUST update the unified source view to reflect the newly active source within 2 seconds of selection

**State Management:**
- **FR-013**: System MUST persist the currently active unified source (including whether it's a Snapcast stream or amplifier input) to local storage
- **FR-014**: System MUST restore the last active unified source when the application starts
- **FR-015**: System MUST synchronize the unified source state with real-time updates from Snapcast server (stream changes) and Home Assistant (amplifier state changes)
- **FR-016**: System MUST reconcile conflicts between persisted state and actual device states at startup

**Integration:**
- **FR-017**: System MUST maintain compatibility with existing Snapcast control features (volume, room selection)
- **FR-018**: System MUST maintain compatibility with existing amplifier control features (power, volume)
- **FR-019**: System MUST use existing communication channels (Snapcast JSON-RPC, MQTT for amplifier) without requiring protocol changes

### Key Entities

- **Unified Source**: Represents any selectable audio source, abstracting over Snapcast streams and amplifier inputs. Key attributes include:
  - Source identifier (unique across both Snapcast and amplifier sources)
  - Display name (user-friendly name shown in UI)
  - Source type (internal: Snapcast stream or amplifier input)
  - Active state (whether this source is currently playing)
  - Associated metadata (stream info for Snapcast sources, input details for amplifier sources)

- **Source Collection**: Aggregated view of all available sources. Responsibilities include:
  - Combining Snapcast streams and amplifier inputs
  - Maintaining order and organization of sources
  - Tracking current active source
  - Providing query interface for UI

- **Source Activation Context**: Encapsulates the state required to activate a source. Includes:
  - Target source information
  - Required actions (amplifier input switch, stream selection)
  - Sequencing requirements (e.g., amplifier input must change before stream selection)
  - Rollback information in case of failure

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Users can view all available sources (Snapcast and amplifier) in a single unified interface without navigating between separate pages
- **SC-002**: Users can switch from any source to any other source (whether Snapcast stream or amplifier input) with a single button press
- **SC-003**: Source selection completes successfully 95% of the time, with the unified view reflecting the active source within 2 seconds
- **SC-004**: System correctly handles the multi-step process of switching to Snapcast streams (amplifier input change + stream selection) transparently, without requiring user intervention
- **SC-005**: Application restores the correct active source (whether Snapcast or amplifier input) after restart in 100% of test cases with valid persisted state
- **SC-006**: Users no longer need to understand that Snapcast is an amplifier input source, reducing cognitive load by eliminating a two-tier mental model
- **SC-007**: Navigation within the unified source view responds to user input within 500ms, providing a smooth browsing experience

## Assumptions *(mandatory)*

1. **Amplifier Configuration**: The amplifier has exactly 5 input sources, one of which is dedicated to Snapcast
2. **Snapcast Integration**: The Snapcast server is already integrated and provides stream listing capabilities
3. **Configuration Clarity**: The system configuration clearly identifies which amplifier input corresponds to Snapcast (e.g., via config.toml)
4. **Source Naming**: Source names are sufficiently distinct and user-friendly for display in a unified list
5. **State Synchronization**: Existing state synchronization mechanisms (MQTT for amplifier, JSON-RPC for Snapcast) provide sufficient real-time updates
6. **Display Capacity**: The hardware controller display can accommodate a scrollable list of at least 10 sources
7. **User Understanding**: Users understand the concept of "sources" as things they can listen to, without needing technical knowledge
8. **Amplifier Power**: Source selection assumes the amplifier is powered on, or the system can detect and handle power-off states gracefully

## Dependencies *(mandatory)*

1. **Feature 001-snapcast-controller**: Requires working Snapcast integration for stream listing and selection
2. **Feature 002-homeassistant-amplifier-control**: Requires amplifier control capabilities, including source selection via IR commands
3. **Configuration**: Requires config.toml to specify which amplifier input is connected to Snapcast output
4. **State Persistence**: Requires existing state persistence mechanisms for storing unified source selection
5. **MQTT and JSON-RPC**: Depends on stable communication with Home Assistant (MQTT) and Snapcast server (JSON-RPC)
