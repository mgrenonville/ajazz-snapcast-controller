# Feature Specification: Home Assistant Amplifier Control

**Feature Branch**: `002-homeassistant-amplifier-control`
**Created**: 2025-12-22
**Status**: Draft
**Input**: User description: "I would like to add a new feature on my controller. I have a home assistant running, with a mqtt integration. I would like to add a way on controller to display the state of a power switch that is connected to amplifier, and allow it to be switched on or off. Also, I have a device to control amplifier like source selection, It would be nice to add a page for this kind of controls"

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Monitor and Control Amplifier Power (Priority: P1)

Users need to view the current power state of their amplifier and control it (on/off) directly from the hardware controller without accessing Home Assistant interface.

**Why this priority**: Power control is the fundamental amplifier operation. Without the ability to turn the amplifier on/off, all other controls are useless. This provides immediate value as a standalone feature.

**Independent Test**: Can be fully tested by connecting the controller to Home Assistant, viewing the amplifier power state on the hardware display, toggling the power switch via controller button, and verifying both the display updates and the physical amplifier responds accordingly.

**Acceptance Scenarios**:

1. **Given** the controller is connected to Home Assistant and the amplifier is currently off, **When** the user views the amplifier control page, **Then** the hardware controller displays the current power state as "OFF"
2. **Given** the amplifier is off and displayed on the controller, **When** the user presses the power control button, **Then** the controller sends a command to turn on the amplifier and the display updates to show "ON" within 2 seconds
3. **Given** the amplifier is on, **When** the user presses the power control button, **Then** the controller sends a command to turn off the amplifier and the display updates to show "OFF" within 2 seconds
4. **Given** the amplifier power state changes in Home Assistant (changed by another user or automation), **When** the state update is received, **Then** the controller display updates to reflect the new state within 2 seconds

---

### User Story 2 - Select Amplifier Input Sources (Priority: P2)

Users need to switch between different audio input sources on their amplifier (e.g., CD player, turntable, streaming device, TV audio) using the hardware controller.

**Why this priority**: After ensuring power control works, source selection is the next most common amplifier operation. It builds on the power control foundation and provides additional value for users who frequently switch between input devices.

**Independent Test**: Can be tested by displaying available amplifier sources on the hardware controller, selecting different sources via controller buttons, and verifying the amplifier switches to the selected input and the display reflects the current source.

**Acceptance Scenarios**:

1. **Given** the controller is on the amplifier control page and the amplifier is powered on, **When** the user views the source selection screen, **Then** all available input sources are displayed with the currently active source highlighted
2. **Given** multiple input sources are displayed, **When** the user presses a button corresponding to a different source, **Then** the amplifier switches to that source and the display updates to highlight the newly selected source within 2 seconds
3. **Given** the amplifier source is changed in Home Assistant, **When** the state update is received, **Then** the controller display updates to reflect the currently active source within 2 seconds
4. **Given** the amplifier is powered off, **When** the user attempts to select a source, **Then** the system displays a message indicating the amplifier must be powered on first

---

### User Story 3 - Navigate Between Snapcast and Amplifier Control Pages (Priority: P3)

Users need to switch between controlling their Snapcast audio system and their amplifier settings using page navigation buttons on the hardware controller.

**Why this priority**: This is a convenience feature that integrates amplifier controls into the existing controller workflow. While valuable, users can still access both Snapcast and amplifier features independently before this is implemented.

**Independent Test**: Can be tested by using page buttons to navigate between existing Snapcast pages (Status, Stream Selection) and the new Amplifier Control page, verifying smooth transitions and correct display of each page's content.

**Acceptance Scenarios**:

1. **Given** the user is viewing the Snapcast status page, **When** the user presses the page button to navigate to amplifier controls, **Then** the controller displays the amplifier power state and control options
2. **Given** the user is viewing the amplifier control page, **When** the user presses the page button to navigate back to Snapcast, **Then** the controller displays the Snapcast status page
3. **Given** the user is navigating between pages, **When** a state change occurs on any system (Snapcast or amplifier), **Then** the display updates appropriately if that page is currently visible

---

### Edge Cases

- What happens when Home Assistant becomes unreachable while the controller is displaying amplifier state?
- How does the system handle the situation when the user rapidly toggles amplifier power multiple times?
- What occurs when the amplifier is controlled by another device/user while the controller is displaying its state?
- How does the system respond when Home Assistant reports an error executing an amplifier command?
- What happens when the available amplifier sources change in Home Assistant configuration?
- How does the controller handle connection loss to Home Assistant during a power toggle or source change operation?
- What occurs when the amplifier responds slower than expected to commands?

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: System MUST display the current power state (on/off) of the amplifier on the hardware controller
- **FR-002**: System MUST allow users to toggle amplifier power (on/off) using a button on the hardware controller
- **FR-003**: System MUST send power control commands to Home Assistant when the user toggles the power button
- **FR-004**: System MUST display all available amplifier input sources on the hardware controller
- **FR-005**: System MUST allow users to select different input sources using buttons on the hardware controller
- **FR-006**: System MUST send source selection commands to Home Assistant when the user selects a source
- **FR-007**: System MUST receive real-time state updates from Home Assistant when amplifier power or source changes
- **FR-008**: System MUST update the hardware controller display within 2 seconds when amplifier state changes
- **FR-009**: System MUST provide visual feedback on the hardware controller when commands are sent to Home Assistant
- **FR-010**: System MUST provide a dedicated page on the hardware controller for amplifier controls, accessible via page navigation buttons
- **FR-011**: System MUST display connection status to Home Assistant on the amplifier control page
- **FR-012**: System MUST handle connection failures to Home Assistant gracefully and display clear error messages on the hardware controller
- **FR-013**: System MUST persist Home Assistant connection settings between application sessions
- **FR-014**: System MUST validate commands before sending to Home Assistant (e.g., prevent source changes when amplifier is off)
- **FR-015**: System MUST queue commands sent to Home Assistant if connectivity is temporarily unavailable and retry when connection is restored

### Assumptions

- Home Assistant is accessible via network connection from the host running the controller application
- Home Assistant MQTT integration is properly configured and operational
- The amplifier power switch entity exists in Home Assistant and reports its state
- The amplifier source selection capability exists in Home Assistant as a controllable entity
- Home Assistant entities for amplifier control use standard naming conventions (e.g., switch.amplifier_power, input_select.amplifier_source)
- The hardware controller has sufficient buttons and screens to accommodate both Snapcast and amplifier controls
- Network latency between controller and Home Assistant is typically under 100ms
- State updates from Home Assistant are pushed in real-time (not polled)
- Only one controller will be used to manage the amplifier at a time
- Users have basic understanding of their amplifier's input sources

### Key Entities

- **Amplifier Power State**: Represents the current on/off state of the amplifier, includes state value (on/off), last updated timestamp, and availability status
- **Amplifier Input Source**: Represents an available audio input on the amplifier, includes source identifier, display name, and currently selected state
- **Home Assistant Connection**: Represents the connection to Home Assistant system, includes connection state, broker address, authentication credentials, and last successful communication timestamp
- **Amplifier Control Page**: Represents the display layout on hardware controller showing amplifier power controls and source selection options

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Users can view amplifier power state on the hardware controller within 3 seconds of navigating to the amplifier control page
- **SC-002**: Users can toggle amplifier power with a single button press, and the amplifier responds within 3 seconds
- **SC-003**: Controller display updates to reflect amplifier state changes within 2 seconds of changes occurring in Home Assistant
- **SC-004**: Users can select amplifier input sources from the hardware controller, and the amplifier switches within 3 seconds
- **SC-005**: 95% of power and source control commands complete successfully when Home Assistant is reachable
- **SC-006**: Connection failures to Home Assistant are detected and displayed to the user within 5 seconds
- **SC-007**: Users can navigate between Snapcast and amplifier control pages within 1 second using page buttons
- **SC-008**: Application maintains connection to Home Assistant with 99% uptime during normal operation
