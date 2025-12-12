# Feature Specification: Snapcast Controller Application

**Feature Branch**: `001-snapcast-controller`
**Created**: 2025-12-12
**Status**: Draft
**Input**: User description: "Create a rust application using rust crate `ajazz_sdk` to control a snapcast server using it's API with `snapcast_control` rust crate"

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Connect to Room's Audio Client (Priority: P1)

Users need to connect a physical hardware controller to an audio server and configure it to control a specific room's audio playback.

**Why this priority**: Without server connectivity and room assignment, the controller cannot manage audio playback. This is the foundational capability that enables all other features.

**Independent Test**: Can be fully tested by connecting the hardware controller via USB, initiating a connection to a running audio server, assigning it to a specific room, and verifying successful connection status is displayed on the hardware controller's screens without performing any audio operations.

**Acceptance Scenarios**:

1. **Given** a hardware controller is connected via USB and an audio server is running and accessible, **When** the user configures the connection and assigns the controller to a specific room, **Then** the application establishes a connection and displays the room's status on the hardware screens
2. **Given** the user has previously connected and configured a controller for a room, **When** the application starts with the hardware controller connected, **Then** it automatically reconnects to the server and displays the assigned room's status
3. **Given** an invalid server address or unreachable server, **When** the user attempts to connect, **Then** the hardware controller displays a clear error message on its screens explaining the connection failure
4. **Given** the hardware controller is not connected, **When** the application starts, **Then** it waits for the controller to be connected and displays waiting status

---

### User Story 2 - Monitor Room Audio Status (Priority: P2)

Users need to view real-time status of their room's audio playback on the hardware controller's screens, including what's playing, volume level, and playback state.

**Why this priority**: After establishing connectivity, users need visibility into their room's current audio state before making control decisions.

**Independent Test**: Can be tested by connecting to a server with active playback in the assigned room and verifying that current status information is displayed on the hardware controller's screens accurately and updates in real-time.

**Acceptance Scenarios**:

1. **Given** a successful server connection with active audio playback in the room, **When** the user views the controller screens, **Then** the hardware displays current playback information including stream name, volume level, and mute status
2. **Given** audio playback state changes in the room on the server, **When** the change occurs, **Then** the hardware controller updates the displayed status on its screens within 2 seconds
3. **Given** the room is assigned to an audio stream, **When** the user views the status, **Then** the controller screens show the currently playing stream and its metadata

---

### User Story 3 - Control Room Audio Playback (Priority: P3)

Users need to control their room's audio playback using the hardware controller's physical knobs and buttons to adjust volume, mute/unmute, and switch between audio streams.

**Why this priority**: Control operations build upon connectivity and monitoring, allowing users to actively manage their room's audio playback through intuitive physical controls.

**Independent Test**: Can be tested by rotating knobs, pressing buttons on the hardware controller, and verifying the audio server executes the commands correctly and the room's audio responds accordingly.

**Acceptance Scenarios**:

1. **Given** a connected room audio client, **When** the user rotates a knob on the controller, **Then** the audio server updates the room's volume level and the change is audible
2. **Given** a connected room audio client, **When** the user presses a button to toggle mute, **Then** the room's audio output is muted or unmuted accordingly
3. **Given** multiple audio streams available, **When** the user presses a button to select a different stream, **Then** the room switches to playing the selected stream
4. **Given** the user operates a control (knob or button), **When** the command fails on the server, **Then** the hardware controller displays a descriptive error message on its screens

---

### Edge Cases

- What happens when the audio server becomes unreachable during an active session?
- How does the application handle commands when the room's audio client becomes disconnected?
- What occurs when the user rotates multiple knobs or presses multiple buttons in rapid succession?
- How does the application respond when the server state changes while a knob/button command is in progress?
- What happens when the hardware controller is disconnected (USB unplugged) during operation?
- How does the application handle reconnection when the hardware controller is plugged back in?
- What occurs when the assigned room no longer exists on the server?

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: System MUST establish and maintain a connection to an audio server using configurable host address and port
- **FR-002**: System MUST authenticate with the audio server if credentials are required
- **FR-003**: System MUST detect and connect to the physical hardware controller when connected via USB
- **FR-004**: System MUST allow users to configure which room (audio client) the controller manages
- **FR-005**: System MUST retrieve and display real-time status of the assigned room's audio playback on the hardware controller's screens, including volume, mute state, and stream assignment
- **FR-006**: System MUST respond to knob rotations on the hardware controller to adjust the room's volume level
- **FR-007**: System MUST respond to button presses on the hardware controller to perform actions such as mute/unmute and stream selection
- **FR-008**: System MUST display information on the hardware controller's button screens to show available actions and current state
- **FR-009**: System MUST use page buttons on the hardware controller to navigate between different views or modes
- **FR-010**: System MUST display available audio streams that the room can be assigned to
- **FR-011**: System MUST handle connection failures (server or hardware) gracefully and provide clear error messages on the hardware screens
- **FR-012**: System MUST detect and report when the server connection is lost during operation
- **FR-013**: System MUST detect and handle when the hardware controller is disconnected during operation
- **FR-014**: System MUST persist connection settings and room assignment between application sessions
- **FR-015**: System MUST validate volume levels (0-100 range) and prevent invalid values from being sent to the server

### Assumptions

- Audio server supports standard JSON-RPC or REST API for control operations
- Server broadcasts state changes to connected clients for real-time updates
- Volume levels are represented as percentage values (0-100)
- Host computer running the application has network access to the audio server
- Host computer has USB port for connecting the hardware controller
- Hardware controller is a USB HID device with 3 knobs, 6 buttons with screens, and 3 page buttons
- Each hardware controller is dedicated to controlling a single room
- One controller will be deployed per room in a multi-room audio system
- Single user will operate each controller at a time

### Key Entities

- **Audio Server**: The central server managing multi-room audio distribution, maintains connection state, registry of rooms (clients), and stream information
- **Room (Audio Client)**: Individual playback device in a specific room, has volume level, mute state, name/identifier, and assigned stream
- **Audio Stream**: Source of audio content being distributed, has identifier, name, playback status, and associated metadata
- **Hardware Controller**: Physical USB HID device with 3 rotary knobs, 6 buttons with individual screens, and 3 page buttons used to control audio playback in one room
- **Connection Settings**: Stored configuration for server connectivity and room assignment, includes host address, port, optional authentication credentials, and assigned room identifier

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Users can establish a connection between the hardware controller and audio server within 10 seconds of providing valid connection details
- **SC-002**: Hardware controller screens display initial room status within 5 seconds of successful connection
- **SC-003**: Status updates on hardware screens reflect server-side changes within 2 seconds of the change occurring
- **SC-004**: Control commands (knob rotations, button presses) execute and provide feedback on hardware screens within 500 milliseconds
- **SC-005**: Application handles network interruptions gracefully, displaying connection status on hardware screens and attempting automatic reconnection
- **SC-006**: Application handles hardware controller disconnection and reconnection without requiring restart
- **SC-007**: 95% of control operations complete successfully on first attempt when server and hardware are connected
- **SC-008**: Application startup and hardware controller detection complete within 3 seconds on typical hardware
