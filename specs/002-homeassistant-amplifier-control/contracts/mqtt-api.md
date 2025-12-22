# MQTT API Contract: Home Assistant Integration

**Feature**: 002-homeassistant-amplifier-control
**Protocol**: MQTT v3.1.1 / v5.0
**Date**: 2025-12-22

## Overview

This document defines the MQTT API contract between the Snapcast hardware controller and Home Assistant. It specifies topic patterns, message formats, QoS levels, and expected behaviors for amplifier control integration.

## Connection Parameters

### Broker Connection

```
Protocol: MQTT (unencrypted for local network)
Host: Configurable (e.g., 192.168.1.100)
Port: Configurable (default 1883)
Client ID: "snapcast-controller-{random_suffix}"
Clean Session: true
Keep Alive: 60 seconds
```

### Authentication

```
Username: Optional (configured in config.toml)
Password: Optional (configured in config.toml)
TLS/SSL: Not required (local network trusted)
```

### QoS Levels

- **Subscribe**: QoS 1 (at least once delivery)
- **Publish Commands**: QoS 1 (at least once delivery)
- **Rationale**: Ensures commands and state updates are not lost during network glitches

---

## Topic Patterns

### State Topics (Subscribe)

Controller subscribes to these topics to receive entity state updates:

#### Power Switch State

```
Topic: homeassistant/switch/{object_id}/state
Payload: "ON" | "OFF"
QoS: 1
Retained: true (last known state)
```

**Example**:
```
Topic: homeassistant/switch/amplifier_power/state
Payload: "ON"
```

**Behavior**:
- Retained message ensures controller receives current state on subscription
- Payload is case-insensitive ("on", "ON", "On" all valid)
- Controller normalizes to uppercase for comparison

#### Input Select State

```
Topic: homeassistant/input_select/{object_id}/state
OR
Topic: homeassistant/select/{object_id}/state
Payload: {source_name}
QoS: 1
Retained: true
```

**Example**:
```
Topic: homeassistant/input_select/amplifier_source/state
Payload: "CD Player"
```

**Behavior**:
- Payload is plain text string (source name)
- Controller stores as-is, displays truncated if >10 characters
- Home Assistant publishes when source changes

#### Entity Availability (Optional)

```
Topic: homeassistant/{component}/{object_id}/availability
Payload: "online" | "offline"
QoS: 1
Retained: true
```

**Example**:
```
Topic: homeassistant/switch/amplifier_power/availability
Payload: "online"
```

**Behavior**:
- If present, controller monitors availability
- "offline" → display "Unavailable" on hardware
- "online" → display normal state

---

### Command Topics (Publish)

Controller publishes to these topics to send commands to Home Assistant:

#### Power Switch Command

```
Topic: homeassistant/switch/{object_id}/set
Payload: "ON" | "OFF"
QoS: 1
Retained: false (commands are not retained)
```

**Example**:
```
Topic: homeassistant/switch/amplifier_power/set
Payload: "ON"
```

**Behavior**:
- Controller publishes desired state
- Home Assistant processes command, updates entity state
- State update published back to state topic
- Round-trip expected: <500ms

#### Input Select Command

```
Topic: homeassistant/input_select/{object_id}/set
OR
Topic: homeassistant/select/{object_id}/set
Payload: {source_name}
QoS: 1
Retained: false
```

**Example**:
```
Topic: homeassistant/input_select/amplifier_source/set
Payload: "Optical Input"
```

**Behavior**:
- Payload must match one of available sources exactly
- Home Assistant validates source is in options list
- Invalid source → no state change, no error feedback
- Valid source → state update published to state topic

---

### Discovery Topics (Optional, Read-Only)

Controller MAY subscribe to discovery topics to auto-detect entity configuration:

```
Topic: homeassistant/{component}/{node_id}/{object_id}/config
Payload: JSON configuration
QoS: 1
Retained: true
```

**Example**:
```
Topic: homeassistant/select/amplifier/source/config
Payload: {
  "name": "Amplifier Source",
  "state_topic": "homeassistant/select/amplifier_source/state",
  "command_topic": "homeassistant/select/amplifier_source/set",
  "options": ["CD Player", "Optical Input", "AUX", "TV"],
  "unique_id": "amplifier_source_selector",
  "device": {
    "identifiers": ["amplifier_01"],
    "name": "Living Room Amplifier"
  }
}
```

**Behavior** (if implemented):
- Parse `options` array for available sources
- Use `state_topic` and `command_topic` for dynamic topic construction
- Fallback to configured entity IDs if discovery not available

**Implementation Note**: Discovery is optional per Simplicity First principle. Initial implementation uses hardcoded entity IDs from config file. Discovery can be added in future iteration if needed.

---

## Message Formats

### State Messages

**Power Switch**:
```json
{
  "topic": "homeassistant/switch/amplifier_power/state",
  "payload": "ON",
  "qos": 1,
  "retain": true
}
```

**Input Select**:
```json
{
  "topic": "homeassistant/input_select/amplifier_source/state",
  "payload": "CD Player",
  "qos": 1,
  "retain": true
}
```

### Command Messages

**Power On**:
```json
{
  "topic": "homeassistant/switch/amplifier_power/set",
  "payload": "ON",
  "qos": 1,
  "retain": false
}
```

**Power Off**:
```json
{
  "topic": "homeassistant/switch/amplifier_power/set",
  "payload": "OFF",
  "qos": 1,
  "retain": false
}
```

**Select Source**:
```json
{
  "topic": "homeassistant/input_select/amplifier_source/set",
  "payload": "Optical Input",
  "qos": 1,
  "retain": false
}
```

---

## Connection Lifecycle

### Initial Connection

```
1. Connect to MQTT broker
   - CONNECT packet with client ID, optional credentials
   - Clean session = true (no persistent subscriptions)

2. Broker responds CONNACK
   - Return code 0 = success
   - Return code 1-5 = connection refused (see error handling)

3. Subscribe to state topics
   - SUBSCRIBE to power state topic (QoS 1)
   - SUBSCRIBE to source state topic (QoS 1)
   - SUBSCRIBE to availability topics (QoS 1, optional)

4. Broker responds SUBACK
   - Granted QoS for each subscription

5. Receive retained messages
   - Broker sends last retained state messages
   - Controller updates ApplicationState

6. Emit BrokerConnected event
   - Main event loop updates UI
```

### Graceful Disconnection

```
1. Send DISCONNECT packet
2. Close TCP connection
3. Emit BrokerDisconnected event
```

### Connection Lost (Unexpected)

```
1. TCP connection drops
2. rumqttc client detects disconnect
3. Emit BrokerDisconnected event
4. Enter reconnection loop with exponential backoff
   - Retry after 1s, 2s, 4s, 8s, 16s, 32s (max 60s)
5. On successful reconnect, repeat Initial Connection flow
```

---

## Error Handling

### Connection Errors

| Scenario | CONNACK Code | Behavior |
|----------|--------------|----------|
| Successful connection | 0 | Proceed to subscribe |
| Unacceptable protocol version | 1 | Log error, do not retry (config issue) |
| Identifier rejected | 2 | Generate new random client ID, retry |
| Server unavailable | 3 | Retry with exponential backoff |
| Bad username or password | 4 | Log error, do not retry (config issue) |
| Not authorized | 5 | Log error, do not retry (config issue) |

### Subscription Errors

| Scenario | SUBACK Code | Behavior |
|----------|-------------|----------|
| Granted QoS 0 | 0 | Accept (degrade to QoS 0) |
| Granted QoS 1 | 1 | Accept |
| Granted QoS 2 | 2 | Accept (upgrade to QoS 2) |
| Subscription failed | 128 | Log error, mark entity as unavailable |

### State Update Errors

| Scenario | Behavior |
|----------|----------|
| Receive unknown entity ID | Log warning, ignore message |
| Receive invalid payload format | Log error, keep previous state |
| Timeout (no state update after 10s) | Mark entity availability as Unknown |
| Receive stale timestamp | Accept (broker may be catching up) |

### Command Errors

| Scenario | Behavior |
|----------|----------|
| Publish fails (disconnected) | Queue command, retry on reconnect |
| Publish times out | Mark as failed, display error to user |
| No state update within 5s after publish | Display timeout error |
| State update contradicts command | Accept new state (Home Assistant rejected command) |

---

## API Invariants

These conditions must ALWAYS hold:

1. **State Consistency**:
   - State topic payload matches entity's actual state in Home Assistant
   - Controller state reflects last received MQTT message

2. **Command Acknowledgment**:
   - Every command publish receives state update within 5 seconds
   - OR connection is marked as failed/unavailable

3. **Availability Synchronization**:
   - Entity marked "offline" → no state updates received
   - Entity marked "online" → state updates resume

4. **Topic Uniqueness**:
   - Each entity has unique state and command topics
   - No topic collisions between entities

5. **Idempotency**:
   - Publishing same command multiple times has same effect as once
   - State updates are idempotent (setting to current value is no-op)

---

## Performance Guarantees

### Latency

- **Publish latency**: <50ms (local network)
- **State update latency**: <200ms (Home Assistant processing + MQTT)
- **Total round-trip**: <500ms (command → publish → HA process → state update)

### Throughput

- **Commands**: Max 10/second (limited by human interaction)
- **State updates**: Max 100/second (HA can burst during startup)
- **Bandwidth**: <1KB/sec average, <10KB/sec peak

### Availability

- **Target uptime**: 99% (tolerates brief network glitches)
- **Reconnect time**: <5 seconds (exponential backoff)
- **Max reconnect delay**: 60 seconds

---

## Security Considerations

### Authentication

- Username/password stored in config file
- Config file permissions: 0600 (owner read/write only)
- No password encryption (local network trusted)

### Authorization

- MQTT broker ACLs (optional, configured on broker):
  - Allow client to subscribe: `homeassistant/+/{object_id}/state`
  - Allow client to publish: `homeassistant/+/{object_id}/set`
  - Deny client: `homeassistant/+/{object_id}/config` (read-only)

### Data Privacy

- All communication unencrypted (local network)
- No sensitive user data transmitted
- Source names and entity IDs are not confidential

### Attack Mitigation

- **Topic injection**: Client validates entity IDs before subscribing
- **Message flooding**: rumqttc has built-in rate limiting
- **Malicious broker**: Client validates message formats, ignores invalid

---

## Example Session

### Full Connection and Command Flow

```
1. Client → Broker: CONNECT (client_id="snapcast-controller-abc123")
2. Broker → Client: CONNACK (return_code=0)

3. Client → Broker: SUBSCRIBE
   - homeassistant/switch/amplifier_power/state (QoS 1)
   - homeassistant/input_select/amplifier_source/state (QoS 1)

4. Broker → Client: SUBACK (granted=[1, 1])

5. Broker → Client: PUBLISH (retained)
   Topic: homeassistant/switch/amplifier_power/state
   Payload: "OFF"
   QoS: 1

6. Client → Broker: PUBACK (acknowledge QoS 1 message)

7. Broker → Client: PUBLISH (retained)
   Topic: homeassistant/input_select/amplifier_source/state
   Payload: "CD Player"
   QoS: 1

8. Client → Broker: PUBACK

[User presses power button on hardware controller]

9. Client → Broker: PUBLISH
   Topic: homeassistant/switch/amplifier_power/set
   Payload: "ON"
   QoS: 1

10. Broker → Client: PUBACK

[Home Assistant processes command, updates entity state]

11. Broker → Client: PUBLISH
    Topic: homeassistant/switch/amplifier_power/state
    Payload: "ON"
    QoS: 1

12. Client → Broker: PUBACK

[Display updates on hardware controller]
```

**Timeline**:
- Steps 1-8: Connection and initial state sync (~500ms)
- Steps 9-10: Command publish (~20ms)
- Step 11: Home Assistant processing + state publish (~200ms)
- Step 12: Acknowledge and display update (~50ms)
- **Total user-perceived latency**: ~270ms (well under 500ms requirement)

---

## Validation Checklist

Before implementation:

- [ ] MQTT broker address and port configurable
- [ ] Optional username/password authentication supported
- [ ] Entity IDs configurable in config.toml
- [ ] State topics follow `homeassistant/{component}/{object_id}/state` pattern
- [ ] Command topics follow `homeassistant/{component}/{object_id}/set` pattern
- [ ] QoS 1 used for all subscriptions and command publishes
- [ ] Retained flag set for state topics, not set for command topics
- [ ] Connection errors trigger exponential backoff reconnection
- [ ] State updates processed and emitted as events to main loop
- [ ] Command validation prevents invalid publishes
- [ ] Timeout detection for commands with no state update
- [ ] Graceful shutdown disconnects MQTT client cleanly

---

## Future Extensions (Not Implemented)

Per Simplicity First principle, these are NOT implemented in initial version:

- **TLS/SSL encryption**: Add when deploying over untrusted network
- **MQTT v5 features**: Request/response pattern, user properties
- **Last Will and Testament**: Notify HA when controller disconnects
- **Discovery auto-configuration**: Parse config topics for entity metadata
- **Multi-entity support**: Control multiple amplifiers
- **Diagnostic topics**: Publish controller health metrics

These can be added in future iterations if needed.

---

## Summary

This MQTT API contract defines:

- **Connection parameters**: Standard MQTT 3.1.1, QoS 1, optional auth
- **Topic patterns**: HomeAssistant standard topics for switches and input selects
- **Message formats**: Plain text payloads (ON/OFF for switch, source name for select)
- **Error handling**: Exponential backoff, timeout detection, graceful degradation
- **Performance targets**: <500ms round-trip, 99% uptime
- **Security model**: Username/password auth, local network trusted

The contract is minimal, focusing on essential functionality per Simplicity First principle.
