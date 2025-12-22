# Quickstart: Home Assistant Amplifier Control Implementation

**Feature**: 002-homeassistant-amplifier-control
**Date**: 2025-12-22
**Target Audience**: Developers implementing this feature

## Overview

This guide provides a step-by-step walkthrough for implementing Home Assistant amplifier control in the Ajazz Snapcast controller. Follow these steps to add MQTT-based power and source selection for an amplifier.

## Prerequisites

Before starting implementation:

- [x] Feature specification reviewed (`spec.md`)
- [x] Implementation plan reviewed (`plan.md`)
- [x] Data model understood (`data-model.md`)
- [x] MQTT API contract reviewed (`contracts/mqtt-api.md`)
- [x] Rust development environment set up (stable toolchain)
- [x] Existing Snapcast controller codebase functional
- [x] Home Assistant instance with MQTT integration available for testing

## Implementation Roadmap

The feature is broken into 3 user stories (independently testable):

1. **P1 - Monitor and Control Amplifier Power** (MVP)
   - Display power state on hardware
   - Toggle power with button press
   - Testing: Can demo power control without source selection

2. **P2 - Select Amplifier Input Sources**
   - Display available sources
   - Switch sources with button press
   - Testing: Can demo source selection independently

3. **P3 - Navigate Between Snapcast and Amplifier Pages**
   - Add page navigation
   - Integrate with existing Snapcast pages
   - Testing: Can demo seamless navigation

**Implementation Strategy**: Build P1 first (MVP), test thoroughly, then add P2, then P3.

---

## Phase 1: Setup and Configuration (P1 Foundation)

### Step 1.1: Add Dependencies

Edit `Cargo.toml` to add MQTT client:

```toml
[dependencies]
# Existing dependencies...
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }

# NEW: MQTT client for Home Assistant
rumqttc = "0.24"
```

Run `cargo build` to fetch new dependency.

### Step 1.2: Create Home Assistant Module

Create new module structure:

```bash
mkdir -p src/homeassistant
touch src/homeassistant/mod.rs
touch src/homeassistant/client.rs
touch src/homeassistant/types.rs
touch src/homeassistant/commands.rs
```

### Step 1.3: Extend Configuration

Edit `src/config/settings.rs`:

```rust
// Add to existing file

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ConnectionSettings {
    pub server: ServerConfig,
    pub room: RoomConfig,
    pub homeassistant: Option<HomeAssistantConfig>,  // NEW
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HomeAssistantConfig {
    pub broker_address: String,
    pub broker_port: u16,
    pub username: Option<String>,
    pub password: Option<String>,
    pub amplifier: AmplifierEntityConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AmplifierEntityConfig {
    pub power_entity: String,
    pub source_entity: String,
}
```

Update validation in `ConnectionSettings::validate()`:

```rust
if let Some(ha_config) = &self.homeassistant {
    anyhow::ensure!(
        !ha_config.broker_address.is_empty(),
        "Home Assistant broker address cannot be empty"
    );
    anyhow::ensure!(
        ha_config.broker_port > 0,
        "Home Assistant broker port must be greater than 0"
    );
    // Validate entity IDs match pattern domain.object_id
    anyhow::ensure!(
        ha_config.amplifier.power_entity.contains('.'),
        "Invalid power entity ID format"
    );
    anyhow::ensure!(
        ha_config.amplifier.source_entity.contains('.'),
        "Invalid source entity ID format"
    );
}
```

### Step 1.4: Update Example Config

Create or update `config.toml.example`:

```toml
[server]
address = "192.168.1.50"
port = 1705

[room]
client_id = "living_room"

# NEW: Home Assistant configuration (optional)
[homeassistant]
broker_address = "192.168.1.100"
broker_port = 1883
# username = "controller"  # uncomment if auth required
# password = "secret"

[homeassistant.amplifier]
power_entity = "switch.amplifier_power"
source_entity = "input_select.amplifier_source"
```

**Test checkpoint**: Run `cargo build`. Should compile without errors.

---

## Phase 2: Data Types and Events (P1)

### Step 2.1: Define Home Assistant Types

Edit `src/homeassistant/types.rs`:

```rust
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Amplifier state as known by controller
#[derive(Debug, Clone)]
pub struct AmplifierState {
    pub power_on: Option<bool>,
    pub current_source: Option<String>,
    pub available_sources: Vec<String>,
    pub last_updated: Instant,
    pub availability: EntityAvailability,
}

impl AmplifierState {
    pub fn new() -> Self {
        Self {
            power_on: None,
            current_source: None,
            available_sources: Vec::new(),
            last_updated: Instant::now(),
            availability: EntityAvailability::Unknown,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityAvailability {
    Available,
    Unavailable,
    Unknown,
}

/// Events emitted by Home Assistant client
#[derive(Debug, Clone)]
pub enum HomeAssistantEvent {
    BrokerConnected,
    BrokerDisconnected,
    EntityStateChanged { entity_id: String, state: String },
    CommandAcknowledged { entity_id: String },
    CommandFailed { entity_id: String, error: String },
}

/// Commands sent to Home Assistant client
#[derive(Debug, Clone)]
pub enum HomeAssistantCommand {
    TogglePower,
    SetPower { on: bool },
    SelectSource { source_name: String },
    Disconnect,
}
```

### Step 2.2: Extend Application State

Edit `src/controller/state.rs`:

```rust
use crate::homeassistant::types::{AmplifierState, HomeAssistantEvent};

pub struct ApplicationState {
    // Existing fields...
    pub config: ConnectionSettings,
    pub room: Option<RoomState>,
    pub current_page: PageView,

    // NEW fields
    pub homeassistant_connected: bool,
    pub amplifier: Option<AmplifierState>,
}

impl ApplicationState {
    pub fn new(config: ConnectionSettings) -> Self {
        Self {
            // Existing initialization...
            homeassistant_connected: false,
            amplifier: Some(AmplifierState::new()),
        }
    }

    /// Handle Home Assistant events
    /// Returns true if screen refresh needed
    pub fn handle_homeassistant_event(&mut self, event: HomeAssistantEvent) -> bool {
        match event {
            HomeAssistantEvent::BrokerConnected => {
                self.homeassistant_connected = true;
                true  // Refresh to show "Connected"
            }
            HomeAssistantEvent::BrokerDisconnected => {
                self.homeassistant_connected = false;
                if let Some(amp) = &mut self.amplifier {
                    amp.power_on = None;
                    amp.current_source = None;
                    amp.availability = EntityAvailability::Unknown;
                }
                true  // Refresh to show "Disconnected"
            }
            HomeAssistantEvent::EntityStateChanged { entity_id, state } => {
                if let Some(amp) = &mut self.amplifier {
                    // Parse entity_id to determine which entity changed
                    if entity_id.contains("power") {
                        let was_on = amp.power_on;
                        amp.power_on = match state.to_uppercase().as_str() {
                            "ON" => Some(true),
                            "OFF" => Some(false),
                            _ => None,
                        };
                        amp.last_updated = Instant::now();
                        amp.power_on != was_on  // Refresh if changed
                    } else if entity_id.contains("source") {
                        let was_source = amp.current_source.clone();
                        amp.current_source = Some(state);
                        amp.last_updated = Instant::now();
                        was_source != amp.current_source  // Refresh if changed
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            _ => false,
        }
    }
}
```

### Step 2.3: Extend PageView Enum

Edit `src/controller/state.rs`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageView {
    Status,
    StreamSelection,
    Settings,
    AmplifierControl,  // NEW
}
```

**Test checkpoint**: Run `cargo build`. Should compile.

---

## Phase 3: MQTT Client Implementation (P1 Core)

### Step 3.1: Implement MQTT Client

Edit `src/homeassistant/client.rs`:

```rust
use rumqttc::{AsyncClient, Event, MqttOptions, Packet, QoS};
use tokio::sync::mpsc;
use std::time::Duration;

use crate::config::settings::HomeAssistantConfig;
use crate::homeassistant::types::{HomeAssistantCommand, HomeAssistantEvent};

pub struct MqttClient {
    config: HomeAssistantConfig,
    client: AsyncClient,
    event_tx: mpsc::UnboundedSender<HomeAssistantEvent>,
}

impl MqttClient {
    pub async fn connect(
        config: HomeAssistantConfig,
        event_tx: mpsc::UnboundedSender<HomeAssistantEvent>,
    ) -> Result<(Self, rumqttc::EventLoop), Box<dyn std::error::Error>> {
        // Create MQTT options
        let mut mqtt_options = MqttOptions::new(
            format!("snapcast-controller-{}", rand::random::<u32>()),
            &config.broker_address,
            config.broker_port,
        );

        if let (Some(username), Some(password)) = (&config.username, &config.password) {
            mqtt_options.set_credentials(username, password);
        }

        mqtt_options.set_keep_alive(Duration::from_secs(60));
        mqtt_options.set_clean_session(true);

        // Create client and event loop
        let (client, event_loop) = AsyncClient::new(mqtt_options, 10);

        Ok((
            Self {
                config,
                client,
                event_tx,
            },
            event_loop,
        ))
    }

    /// Subscribe to entity state topics
    pub async fn subscribe_to_entities(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Extract object IDs from entity IDs
        let power_object_id = self.config.amplifier.power_entity
            .split('.').nth(1).unwrap_or("amplifier_power");
        let source_object_id = self.config.amplifier.source_entity
            .split('.').nth(1).unwrap_or("amplifier_source");

        // Subscribe to power state
        let power_topic = format!("homeassistant/switch/{}/state", power_object_id);
        self.client.subscribe(&power_topic, QoS::AtLeastOnce).await?;

        // Subscribe to source state
        let source_topic = format!("homeassistant/input_select/{}/state", source_object_id);
        self.client.subscribe(&source_topic, QoS::AtLeastOnce).await?;

        eprintln!("Subscribed to: {} and {}", power_topic, source_topic);
        Ok(())
    }

    /// Handle incoming MQTT packet
    pub fn handle_packet(&self, packet: Packet) {
        if let Packet::Publish(publish) = packet {
            let topic = publish.topic.clone();
            let payload = String::from_utf8_lossy(&publish.payload).to_string();

            eprintln!("Received: {} -> {}", topic, payload);

            // Determine entity_id from topic
            let entity_id = if topic.contains("/switch/") {
                self.config.amplifier.power_entity.clone()
            } else if topic.contains("/input_select/") || topic.contains("/select/") {
                self.config.amplifier.source_entity.clone()
            } else {
                return;  // Unknown topic
            };

            // Emit state changed event
            let _ = self.event_tx.send(HomeAssistantEvent::EntityStateChanged {
                entity_id,
                state: payload,
            });
        }
    }

    /// Execute command
    pub async fn execute_command(
        &self,
        command: HomeAssistantCommand,
        current_power_state: Option<bool>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        match command {
            HomeAssistantCommand::TogglePower => {
                let target_state = match current_power_state {
                    Some(true) => "OFF",
                    Some(false) => "ON",
                    None => return Err("Power state unknown, cannot toggle".into()),
                };
                self.publish_power_command(target_state).await?;
            }
            HomeAssistantCommand::SetPower { on } => {
                let target_state = if on { "ON" } else { "OFF" };
                self.publish_power_command(target_state).await?;
            }
            HomeAssistantCommand::SelectSource { source_name } => {
                self.publish_source_command(&source_name).await?;
            }
            HomeAssistantCommand::Disconnect => {
                self.client.disconnect().await?;
            }
        }
        Ok(())
    }

    async fn publish_power_command(&self, state: &str) -> Result<(), Box<dyn std::error::Error>> {
        let object_id = self.config.amplifier.power_entity
            .split('.').nth(1).unwrap_or("amplifier_power");
        let topic = format!("homeassistant/switch/{}/set", object_id);

        self.client.publish(&topic, QoS::AtLeastOnce, false, state).await?;
        eprintln!("Published: {} -> {}", topic, state);

        let _ = self.event_tx.send(HomeAssistantEvent::CommandAcknowledged {
            entity_id: self.config.amplifier.power_entity.clone(),
        });

        Ok(())
    }

    async fn publish_source_command(&self, source: &str) -> Result<(), Box<dyn std::error::Error>> {
        let object_id = self.config.amplifier.source_entity
            .split('.').nth(1).unwrap_or("amplifier_source");
        let topic = format!("homeassistant/input_select/{}/set", object_id);

        self.client.publish(&topic, QoS::AtLeastOnce, false, source).await?;
        eprintln!("Published: {} -> {}", topic, source);

        let _ = self.event_tx.send(HomeAssistantEvent::CommandAcknowledged {
            entity_id: self.config.amplifier.source_entity.clone(),
        });

        Ok(())
    }
}

/// Main MQTT loop (runs in separate tokio task)
pub async fn mqtt_loop(
    config: HomeAssistantConfig,
    event_tx: mpsc::UnboundedSender<HomeAssistantEvent>,
    mut command_rx: mpsc::UnboundedReceiver<(HomeAssistantCommand, Option<bool>)>,
) -> Result<(), Box<dyn std::error::Error>> {
    let (client, mut event_loop) = MqttClient::connect(config, event_tx.clone()).await?;

    // Wait for connection
    loop {
        match event_loop.poll().await {
            Ok(Event::Incoming(Packet::ConnAck(_))) => {
                eprintln!("Connected to Home Assistant MQTT broker");
                let _ = event_tx.send(HomeAssistantEvent::BrokerConnected);
                break;
            }
            Ok(_) => continue,
            Err(e) => {
                eprintln!("Connection failed: {}", e);
                let _ = event_tx.send(HomeAssistantEvent::BrokerDisconnected);
                tokio::time::sleep(Duration::from_secs(5)).await;
                continue;
            }
        }
    }

    // Subscribe to entities
    if let Err(e) = client.subscribe_to_entities().await {
        eprintln!("Subscription failed: {}", e);
    }

    // Event loop
    loop {
        tokio::select! {
            // Handle incoming MQTT packets
            event = event_loop.poll() => {
                match event {
                    Ok(Event::Incoming(packet)) => {
                        client.handle_packet(packet);
                    }
                    Ok(_) => {},
                    Err(e) => {
                        eprintln!("MQTT error: {}", e);
                        let _ = event_tx.send(HomeAssistantEvent::BrokerDisconnected);
                        tokio::time::sleep(Duration::from_secs(5)).await;
                        // Reconnection logic would go here
                    }
                }
            }

            // Handle outgoing commands
            Some((command, power_state)) = command_rx.recv() => {
                if let Err(e) = client.execute_command(command, power_state).await {
                    eprintln!("Command failed: {}", e);
                    let _ = event_tx.send(HomeAssistantEvent::CommandFailed {
                        entity_id: "unknown".to_string(),
                        error: e.to_string(),
                    });
                }
            }
        }
    }
}
```

### Step 3.2: Wire Module Together

Edit `src/homeassistant/mod.rs`:

```rust
pub mod client;
pub mod types;

pub use client::mqtt_loop;
pub use types::{AmplifierState, EntityAvailability, HomeAssistantCommand, HomeAssistantEvent};
```

Edit `src/main.rs` to add module declaration:

```rust
mod homeassistant;
```

**Test checkpoint**: Run `cargo build`. Should compile without errors.

---

## Phase 4: Main Event Loop Integration (P1)

### Step 4.1: Spawn MQTT Task

Edit `src/main.rs` to spawn Home Assistant task:

```rust
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // ... existing setup code...

    // Existing channels
    let (hardware_event_tx, mut hardware_event_rx) = mpsc::channel(100);
    let (snapcast_event_tx, mut snapcast_event_rx) = mpsc::channel(100);
    let (hardware_command_tx, hardware_command_rx) = watch::channel(None);

    // NEW: Home Assistant channels
    let (ha_event_tx, mut ha_event_rx) = mpsc::unbounded_channel();
    let (ha_command_tx, ha_command_rx) = mpsc::unbounded_channel();

    // Spawn Home Assistant task (if configured)
    if let Some(ha_config) = app_state.config.homeassistant.clone() {
        tokio::spawn(async move {
            if let Err(e) = homeassistant::mqtt_loop(ha_config, ha_event_tx, ha_command_rx).await {
                eprintln!("Home Assistant MQTT loop failed: {}", e);
            }
        });
    }

    // ... existing hardware and snapcast tasks...

    // Main event loop
    loop {
        tokio::select! {
            Some(hw_event) = hardware_event_rx.recv() => {
                handle_hardware_event(&mut app_state, hw_event, &hardware_command_tx, &snapcast_command_tx, &ha_command_tx).await;
            }

            Some(snap_event) = snapcast_event_rx.recv() => {
                handle_snapcast_event(&mut app_state, snap_event).await;
            }

            // NEW: Handle Home Assistant events
            Some(ha_event) = ha_event_rx.recv() => {
                let needs_refresh = app_state.handle_homeassistant_event(ha_event);
                if needs_refresh && app_state.current_page == PageView::AmplifierControl {
                    // TODO: Build and send display update
                }
            }

            _ = tokio::signal::ctrl_c() => {
                println!("\nShutting down...");
                break;
            }
        }
    }

    Ok(())
}
```

### Step 4.2: Handle Hardware Events for Amplifier Page

Update `handle_hardware_event` function in `src/main.rs`:

```rust
async fn handle_hardware_event(
    state: &mut ApplicationState,
    event: HardwareEvent,
    hardware_command_tx: &watch::Sender<Option<HardwareCommand>>,
    snapcast_command_tx: &mpsc::UnboundedSender<SnapcastCommand>,
    ha_command_tx: &mpsc::UnboundedSender<(HomeAssistantCommand, Option<bool>)>,  // NEW
) -> bool {
    match event {
        HardwareEvent::PageButtonPressed { page_id } => {
            match page_id {
                2 => {
                    state.current_page = PageView::AmplifierControl;
                    return true;  // Refresh display
                }
                _ => {
                    // Existing Snapcast page handling
                }
            }
        }

        HardwareEvent::ButtonPressed { button_id } => {
            if state.current_page == PageView::AmplifierControl {
                match button_id {
                    0 => {
                        // Toggle power
                        if let Some(amp) = &state.amplifier {
                            let current_state = amp.power_on;
                            let _ = ha_command_tx.send((
                                HomeAssistantCommand::TogglePower,
                                current_state,
                            ));
                        }
                    }
                    _ => {}
                }
            } else {
                // Existing Snapcast button handling
            }
        }

        _ => {}
    }

    false
}
```

**Test checkpoint**: Run `cargo build`. Should compile.

---

## Phase 5: Display Implementation (P1)

### Step 5.1: Create Amplifier Page Layout

Edit `src/hardware/display.rs`:

```rust
// Add new layout struct

#[derive(Debug, Clone)]
pub struct AmplifierControlPageLayout {
    pub power_status: String,
    pub current_source: String,
    pub ha_connection: String,
}

impl AmplifierControlPageLayout {
    pub fn from_amplifier_state(
        power_on: Option<bool>,
        current_source: Option<&str>,
        ha_connected: bool,
    ) -> Self {
        let power_status = match power_on {
            Some(true) => "ON".to_string(),
            Some(false) => "OFF".to_string(),
            None => "Unknown".to_string(),
        };

        let current_source = current_source.unwrap_or("Unknown").to_string();

        let ha_connection = if ha_connected {
            "Connected".to_string()
        } else {
            "Disconnected".to_string()
        };

        Self {
            power_status,
            current_source,
            ha_connection,
        }
    }
}

impl DisplayManager {
    pub async fn render_amplifier_control_page(
        &self,
        device: &Arc<AsyncAjazz>,
        layout: &AmplifierControlPageLayout,
    ) -> Result<(), HardwareError> {
        // Button 0: Power status
        self.render_amplifier_power_screen(device, 0, &layout.power_status).await?;

        // Button 1: Current source
        self.render_amplifier_source_screen(device, 1, &layout.current_source).await?;

        // Button 3: HA connection
        self.render_ha_connection_screen(device, 3, &layout.ha_connection).await?;

        Ok(())
    }

    async fn render_amplifier_power_screen(
        &self,
        device: &Arc<AsyncAjazz>,
        button: u8,
        power_status: &str,
    ) -> Result<(), HardwareError> {
        let mut image = self.create_blank_image();
        self.draw_top_label(&mut image, "POWER", 12.0, 8);
        self.draw_centered_text(&mut image, power_status, 20.0, 5);
        self.send_image_to_button(device, button, image).await
    }

    async fn render_amplifier_source_screen(
        &self,
        device: &Arc<AsyncAjazz>,
        button: u8,
        source: &str,
    ) -> Result<(), HardwareError> {
        let mut image = self.create_blank_image();
        self.draw_top_label(&mut image, "SOURCE", 10.0, 5);

        let display_source = if source.len() > 10 {
            format!("{}...", &source[0..7])
        } else {
            source.to_string()
        };

        self.draw_centered_text(&mut image, &display_source, 14.0, 5);
        self.send_image_to_button(device, button, image).await
    }

    async fn render_ha_connection_screen(
        &self,
        device: &Arc<AsyncAjazz>,
        button: u8,
        status: &str,
    ) -> Result<(), HardwareError> {
        let mut image = self.create_blank_image();
        self.draw_top_label(&mut image, "HA", 12.0, 8);
        self.draw_centered_text(&mut image, status, 12.0, 5);
        self.send_image_to_button(device, button, image).await
    }
}
```

### Step 5.2: Add Display Command

Edit `src/hardware/events.rs`:

```rust
use crate::hardware::display::{StatusPageLayout, StreamSelectionPageLayout, AmplifierControlPageLayout};

pub enum HardwareCommand {
    UpdateStatusPage(StatusPageLayout),
    UpdateStreamSelectionPage(StreamSelectionPageLayout),
    UpdateAmplifierPage(AmplifierControlPageLayout),  // NEW
    ShowError(String),
}
```

Update display loop in `src/hardware/device.rs`:

```rust
HardwareCommand::UpdateAmplifierPage(layout) => {
    if let Err(e) = display_manager.render_amplifier_control_page(device, &layout).await {
        eprintln!("Failed to update amplifier page: {}", e);
    }
}
```

### Step 5.3: Wire Display Updates

Edit `src/main.rs` to build amplifier page layout:

```rust
fn build_amplifier_display_command(state: &ApplicationState) -> Option<HardwareCommand> {
    use crate::hardware::display::AmplifierControlPageLayout;

    let layout = if let Some(amp) = &state.amplifier {
        AmplifierControlPageLayout::from_amplifier_state(
            amp.power_on,
            amp.current_source.as_deref(),
            state.homeassistant_connected,
        )
    } else {
        AmplifierControlPageLayout::from_amplifier_state(None, None, state.homeassistant_connected)
    };

    Some(HardwareCommand::UpdateAmplifierPage(layout))
}
```

Call in main event loop when HA events received:

```rust
Some(ha_event) = ha_event_rx.recv() => {
    let needs_refresh = app_state.handle_homeassistant_event(ha_event);
    if needs_refresh && app_state.current_page == PageView::AmplifierControl {
        if let Some(command) = build_amplifier_display_command(&app_state) {
            let _ = hardware_command_tx.send(Some(command));
        }
    }
}
```

**Test checkpoint**: Run `cargo build`. Should compile without errors.

---

## Phase 6: End-to-End Testing (P1)

### Test 6.1: Connection Test

1. Configure Home Assistant in `config.toml`:
```toml
[homeassistant]
broker_address = "192.168.1.100"
broker_port = 1883

[homeassistant.amplifier]
power_entity = "switch.amplifier_power"
source_entity = "input_select.amplifier_source"
```

2. Run application:
```bash
cargo run
```

3. Expected console output:
```
Connected to Home Assistant MQTT broker
Subscribed to: homeassistant/switch/amplifier_power/state and ...
Received: homeassistant/switch/amplifier_power/state -> OFF
```

4. Navigate to amplifier control page (press page button 2)

5. Verify display shows:
   - Button 0: "POWER" / "OFF" (or "ON" based on actual state)
   - Button 3: "HA" / "Connected"

### Test 6.2: Power Toggle

1. Press button 0 on hardware controller

2. Expected console output:
```
Published: homeassistant/switch/amplifier_power/set -> ON
Received: homeassistant/switch/amplifier_power/state -> ON
```

3. Verify amplifier physically powers on

4. Verify display updates: Button 0 shows "ON"

5. Press button 0 again to toggle off

6. Verify same flow with "OFF" state

### Test 6.3: State Sync

1. Open Home Assistant UI

2. Toggle amplifier power switch in UI (not from controller)

3. Expected: Controller display updates within 2 seconds

4. Verify button 0 reflects new state

### Test 6.4: Connection Loss

1. Stop MQTT broker (or disconnect network)

2. Expected console output:
```
MQTT error: connection reset
```

3. Verify display shows: Button 3: "HA" / "Disconnected"

4. Verify power state shows "Unknown"

5. Restart MQTT broker

6. Verify automatic reconnection and state sync

---

## Next Steps (P2 and P3)

Once P1 is working:

**P2 - Source Selection**:
1. Parse available sources from Home Assistant (config or discovery)
2. Add source selection sub-page (similar to stream selection)
3. Handle button press to select source
4. Publish source selection command to MQTT

**P3 - Page Navigation**:
1. Extend page button handling to include all pages
2. Test navigation flow: Status → StreamSelection → AmplifierControl → Status

---

## Troubleshooting

### Build Errors

**Issue**: `rumqttc` not found
- **Fix**: Run `cargo build` to fetch dependencies

**Issue**: Module not found errors
- **Fix**: Ensure all `mod.rs` files declare submodules correctly

### Runtime Errors

**Issue**: MQTT connection refused
- **Fix**: Check broker address, port, and firewall settings
- **Fix**: Verify Home Assistant MQTT integration is running

**Issue**: No state updates received
- **Fix**: Verify entity IDs match Home Assistant configuration
- **Fix**: Check topics with MQTT client tool (mosquitto_sub)

**Issue**: Commands not working
- **Fix**: Verify command topics are correct
- **Fix**: Check Home Assistant logs for errors

### Display Issues

**Issue**: Amplifier page not showing
- **Fix**: Verify page button 2 mapping is correct
- **Fix**: Check PageView enum includes AmplifierControl

**Issue**: State not updating on display
- **Fix**: Verify build_amplifier_display_command is called
- **Fix**: Check hardware_command_tx.send() succeeds

---

## Performance Validation

After implementation, validate against success criteria:

- [ ] SC-001: Amplifier state displayed within 3s of page navigation
- [ ] SC-002: Power toggle responds within 3s
- [ ] SC-003: Display updates within 2s of Home Assistant state change
- [ ] SC-005: 95% of commands succeed (test 20+ toggles)
- [ ] SC-006: Connection failures detected within 5s
- [ ] SC-007: Page navigation completes within 1s

---

## Code Review Checklist

Before committing P1:

- [ ] Configuration validated on load
- [ ] MQTT client handles connection errors gracefully
- [ ] State updates parsed correctly (ON/OFF, source names)
- [ ] Commands validated before publishing
- [ ] Display layouts render without errors
- [ ] Event loop doesn't block on any operation
- [ ] No unwrap() or expect() in production paths
- [ ] Error messages logged to stderr for debugging
- [ ] Code follows existing patterns (mirrors Snapcast module structure)

---

## Summary

This quickstart provides a complete implementation path for P1 (Monitor and Control Amplifier Power):

1. ✅ Added rumqttc dependency
2. ✅ Created homeassistant module structure
3. ✅ Extended configuration with Home Assistant settings
4. ✅ Defined data types (AmplifierState, events, commands)
5. ✅ Implemented MQTT client with publish/subscribe
6. ✅ Integrated into main event loop
7. ✅ Added amplifier control page display
8. ✅ Provided testing instructions

After validating P1, proceed with P2 (source selection) and P3 (page navigation) using similar patterns.
