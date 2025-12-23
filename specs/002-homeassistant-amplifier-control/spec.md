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

Users need to switch between different audio input sources on their amplifier (e.g., Phono, CD, Spotify, etc.) using the hardware controller. The controller tracks the selected source locally and sends IR commands via MQTT to change the amplifier's physical input.

**Why this priority**: After ensuring power control works, source selection is the next most common amplifier operation. It builds on the power control foundation and provides additional value for users who frequently switch between input devices.

**Independent Test**: Can be tested by displaying available amplifier sources on the hardware controller, selecting different sources via controller buttons, and verifying the IR commands are sent via MQTT and the display reflects the selected source.

**Acceptance Scenarios**:

1. **Given** the controller is on the amplifier control page and the amplifier is powered on, **When** the user views the source selection screen, **Then** all available input sources (Phono, CD, Spotify, Source 4, Source 5) are displayed with the currently selected source highlighted
2. **Given** multiple input sources are displayed, **When** the user presses a button corresponding to a different source, **Then** the controller sends an IR command via MQTT to the IR Blaster and updates the local state to reflect the newly selected source
3. **Given** the application is restarted, **When** the amplifier control page is displayed, **Then** the last selected source is restored from persisted state
4. **Given** the amplifier is powered off, **When** the user selects a source, **Then** the system updates the local state but may warn that the amplifier must be powered on for the change to take effect

---

### User Story 3 - Adjust Amplifier Volume (Priority: P2)

Users need to increase or decrease the amplifier volume using a rotary knob on the hardware controller. The controller sends IR commands via MQTT to adjust the volume up or down based on knob rotation.

**Why this priority**: Volume control is a fundamental and frequently used amplifier operation. Users need quick access to adjust volume without accessing Home Assistant or the physical amplifier remote.

**Independent Test**: Can be tested by rotating the volume knob on the hardware controller and verifying the IR commands are sent via MQTT to control the amplifier volume.

**Acceptance Scenarios**:

1. **Given** the controller is on the amplifier control page and the amplifier is powered on, **When** the user rotates the volume knob clockwise, **Then** the controller sends IR commands via MQTT to increase the amplifier volume proportionally to the rotation
2. **Given** the controller is on the amplifier control page and the amplifier is powered on, **When** the user rotates the volume knob counter-clockwise, **Then** the controller sends IR commands via MQTT to decrease the amplifier volume proportionally to the rotation
3. **Given** the user rotates the volume knob quickly, **When** multiple rotation steps are detected, **Then** the controller sends multiple volume commands to achieve the desired volume change
4. **Given** the amplifier is powered off, **When** the user attempts to adjust volume, **Then** the system may warn that the amplifier must be powered on for volume control

---

### User Story 4 - Navigate Between Snapcast and Amplifier Control Pages (Priority: P3)

Users need to switch between controlling their Snapcast audio system and their amplifier settings using page navigation buttons on the hardware controller.

**Why this priority**: This is a convenience feature that integrates amplifier controls into the existing controller workflow. While valuable, users can still access both Snapcast and amplifier features independently before this is implemented.

**Independent Test**: Can be tested by using page buttons to navigate between existing Snapcast pages (Status, Stream Selection) and the new Amplifier Control page, verifying smooth transitions and correct display of each page's content.

**Acceptance Scenarios**:

1. **Given** the user is viewing the Snapcast status page, **When** the user presses the page button to navigate to amplifier controls, **Then** the controller displays the amplifier power state and control options
2. **Given** the user is viewing the amplifier control page, **When** the user presses the page button to navigate back to Snapcast, **Then** the controller displays the Snapcast status page
3. **Given** the user is navigating between pages, **When** a state change occurs on any system (Snapcast or amplifier), **Then** the display updates appropriately if that page is currently visible

---

### Edge Cases

- What happens when Home Assistant/MQTT broker becomes unreachable while the controller is displaying amplifier state?
- How does the system handle the situation when the user rapidly toggles amplifier power multiple times?
- What occurs when the amplifier is controlled by another device/user (physical remote, another controller) while the controller is displaying its locally tracked state?
- How does the system respond when MQTT publish fails or the IR Blaster is offline?
- What happens when the user rotates the volume knob very rapidly?
- How does the controller handle connection loss to MQTT broker during a power toggle, source change, or volume adjustment operation?
- What occurs when the amplifier responds slower than expected to IR commands?
- How does the system handle state synchronization when the application restarts and the amplifier's actual state differs from the persisted state?
- What happens if IR commands are sent but the amplifier doesn't respond (e.g., IR receiver blocked)?

## Requirements *(mandatory)*

### Functional Requirements

**Power Control:**
- **FR-001**: System MUST display the current power state (on/off) of the amplifier on the hardware controller based on locally tracked state
- **FR-002**: System MUST allow users to toggle amplifier power (on/off) using a button on the hardware controller
- **FR-003**: System MUST send power control commands via MQTT to Home Assistant when the user toggles the power button
- **FR-004**: System MUST receive real-time power state updates from Home Assistant via MQTT when amplifier power changes
- **FR-005**: System MUST update the hardware controller display within 2 seconds when amplifier power state changes

**Source Selection:**
- **FR-006**: System MUST display all available amplifier input sources (Phono, CD, Spotify, Source 4, Source 5) on the hardware controller
- **FR-007**: System MUST allow users to select different input sources using buttons on the hardware controller
- **FR-008**: System MUST send IR commands via MQTT to the IR Blaster when the user selects a source
- **FR-009**: System MUST maintain local state tracking of the currently selected amplifier source
- **FR-010**: System MUST persist the selected source state between application sessions
- **FR-011**: System MUST update the hardware controller display immediately when the user selects a different source

**Volume Control:**
- **FR-012**: System MUST allow users to adjust volume using a rotary knob on the hardware controller
- **FR-013**: System MUST send volume up/down IR commands via MQTT to the IR Blaster based on knob rotation direction
- **FR-014**: System MUST handle rapid knob rotation by sending appropriate number of volume commands

**General:**
- **FR-015**: System MUST provide visual feedback on the hardware controller when IR commands are sent via MQTT
- **FR-016**: System MUST provide a dedicated page on the hardware controller for amplifier controls, accessible via page navigation buttons
- **FR-017**: System MUST display connection status to MQTT broker on the amplifier control page
- **FR-018**: System MUST handle connection failures to MQTT broker gracefully and display clear error messages on the hardware controller
- **FR-019**: System MUST persist MQTT broker connection settings between application sessions
- **FR-020**: System MUST use the configured MQTT topic for the IR Blaster and the correct payload format for RC5 IR commands
- **FR-021**: System MUST allow configuration of the IR Blaster MQTT topic in the application settings

### Assumptions

- Home Assistant is accessible via network connection from the host running the controller application
- Home Assistant MQTT integration is properly configured and operational
- An IR Blaster device (Tasmota) is configured in Home Assistant and accessible via MQTT
- The IR Blaster MQTT topic is configurable (e.g., `tasmota_XXXXXX/cmnd/irsend`)
- The amplifier power switch entity exists in Home Assistant and reports its state via MQTT
- The IR Blaster can send RC5 protocol IR commands to control the amplifier
- IR commands for source selection use specific data codes: Phono (0xC01), CD (0xC02), Spotify (0xC03), Source 4 (0xC04), Source 5 (0xC05)
- IR commands for volume control use data codes: Volume Up (0xC10), Volume Down (0xC11)
- The hardware controller has sufficient buttons, a rotary knob, and screens to accommodate both Snapcast and amplifier controls
- Network latency between controller and MQTT broker is typically under 100ms
- Power state updates from Home Assistant are pushed in real-time via MQTT (not polled)
- Source selection state is maintained locally by the controller application (not synchronized from Home Assistant)
- Volume level state is not tracked (volume adjustments are send-only IR commands)
- The amplifier responds to IR commands within a reasonable time frame (typically < 1 second)
- Users have basic understanding of their amplifier's input sources

### Key Entities

- **Amplifier Power State**: Represents the current on/off state of the amplifier, includes state value (on/off), last updated timestamp, and availability status (synchronized from Home Assistant via MQTT)
- **Amplifier Input Source**: Represents an available audio input on the amplifier, includes source identifier (Phono, CD, Spotify, Source 4, Source 5), display name, IR command code, and currently selected state (tracked locally)
- **IR Command**: Represents an infrared command to be sent to the amplifier, includes protocol (RC5), bits (12), data code (hex value), and repeat count
- **MQTT Connection**: Represents the connection to the MQTT broker, includes connection state, broker address, authentication credentials, IR Blaster topic, and last successful communication timestamp
- **Volume Control**: Represents volume adjustment capability, includes direction (up/down) and corresponding IR command codes
- **Amplifier Control Page**: Represents the display layout on hardware controller showing amplifier power controls, source selection options, and volume knob feedback

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Users can view amplifier power state on the hardware controller within 3 seconds of navigating to the amplifier control page
- **SC-002**: Users can toggle amplifier power with a single button press, and IR commands are sent via MQTT within 200ms
- **SC-003**: Controller display updates to reflect amplifier power state changes within 2 seconds of changes occurring in Home Assistant
- **SC-004**: Users can select amplifier input sources from the hardware controller, and the display updates immediately with IR commands sent via MQTT within 200ms
- **SC-005**: Users can adjust volume using the rotary knob, and IR commands are sent via MQTT within 100ms per knob step
- **SC-006**: 95% of power, source, and volume control IR commands are successfully published to MQTT when the broker is reachable
- **SC-007**: Selected source state persists across application restarts and is restored within 1 second of startup
- **SC-008**: Connection failures to MQTT broker are detected and displayed to the user within 5 seconds
- **SC-009**: Users can navigate between Snapcast and amplifier control pages within 1 second using page buttons
- **SC-010**: Application maintains connection to MQTT broker with 99% uptime during normal operation
