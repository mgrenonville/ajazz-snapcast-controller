// Snapcast Controller Application
// Controls a single room's audio playback via USB HID hardware controller

use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Duration;

use tokio::sync::{mpsc::unbounded_channel, watch};

use crate::config::settings::ConnectionSettings;
use crate::controller::state::ApplicationState;
use crate::hardware::{
    device::DeviceManager,
    display::{AmplifierControlPageLayout, StatusPageLayout},
    events::{HardwareCommand, HardwareEvent},
};
use crate::homeassistant::types::{HomeAssistantCommand, HomeAssistantEvent};
use crate::snapcast::{
    client::SnapcastClient,
    types::{SnapcastCommand, SnapcastEvent},
};

mod config;
mod controller;
mod hardware;
mod homeassistant;
mod snapcast;

#[tokio::main]
async fn main() {
    println!("Snapcast Controller Application");
    println!("Initialization in progress...");

    // Load configuration (T024)
    let config_path = get_config_path();
    let config = load_config(&config_path);

    println!("Configuration loaded successfully");
    println!("  Server: {}:{}", config.server.address, config.server.port);
    println!("  Room client ID: {}", config.room.client_id);

    // T035: Initialize ApplicationState with loaded config
    let mut app_state = ApplicationState::new(config.clone());

    // Create event channels
    let (hardware_event_tx, mut hardware_event_rx) = unbounded_channel::<HardwareEvent>();
    let (snapcast_event_tx, mut snapcast_event_rx) = unbounded_channel::<SnapcastEvent>();
    let (homeassistant_event_tx, mut homeassistant_event_rx) =
        unbounded_channel::<HomeAssistantEvent>();
    // Watch channel for screen updates - always holds latest state, drops old updates
    let (hardware_command_tx, hardware_command_rx) =
        watch::channel::<Option<HardwareCommand>>(None);
    let (snapcast_command_tx, snapcast_command_rx) = unbounded_channel::<SnapcastCommand>();
    let (homeassistant_command_tx, homeassistant_command_rx) =
        unbounded_channel::<HomeAssistantCommand>();

    // Parse server address
    let server_addr = format!("{}:{}", config.server.address, config.server.port)
        .parse::<SocketAddr>()
        .expect("Invalid server address");

    // T033: Create tokio task for hardware event listening and display updates
    let hardware_manager = std::sync::Arc::new(tokio::sync::Mutex::new(DeviceManager::new(
        hardware_event_tx,
    )));

    // Spawn device connection/health monitoring task
    let hardware_monitor_task = tokio::spawn({
        let manager = hardware_manager.clone();
        async move {
            if let Err(e) =
                hardware::device::device_monitor_loop(manager, Duration::from_millis(10)).await
            {
                eprintln!("Hardware monitor loop failed: {}", e);
            }
        }
    });

    // Spawn display update task
    let hardware_display_task = tokio::spawn({
        let manager = hardware_manager.clone();
        async move {
            if let Err(e) =
                hardware::device::device_display_loop(manager, hardware_command_rx).await
            {
                eprintln!("Hardware display loop failed: {}", e);
            }
        }
    });

    // T034: Create Snapcast client and run in background
    // Note: Due to snapcast_control's State using OnceCell (not Sync),
    // we cannot spawn this as a separate task. Instead, we'll run the
    // connection logic in the main thread.
    let snapcast_client = SnapcastClient::new(
        server_addr,
        config.room.client_id.clone(),
        snapcast_event_tx,
    );
    let snapcast_task = async move {
        let _ = snapcast::client::message_loop(
            snapcast_client,
            Duration::from_secs(2),
            snapcast_command_rx,
        )
        .await;
    };

    // T038: Create Home Assistant MQTT client task (if configured)
    let _homeassistant_task = if let Some(ref ha_config) = config.homeassistant {
        let (mqtt_client, eventloop) = homeassistant::client::MqttClient::new(
            ha_config.clone(),
            homeassistant_event_tx,
            homeassistant_command_rx,
        );

        // Load persisted amplifier state
        if let Err(e) = app_state.load_selected_source() {
            eprintln!("Failed to load amplifier state: {}", e);
        }

        Some(tokio::spawn(async move {
            mqtt_client.run(eventloop).await;
        }))
    } else {
        eprintln!("Home Assistant not configured, skipping MQTT client");
        None
    };

    // T032: Implement main event loop
    println!("Starting main event loop...");

    // Pin the snapcast task so we can poll it
    tokio::pin!(snapcast_task);

    loop {
        tokio::select! {
            // Process hardware events
            Some(hw_event) = hardware_event_rx.recv() => {
                let needs_refresh = handle_hardware_event(&mut app_state, hw_event, &hardware_command_tx, &snapcast_command_tx, &homeassistant_command_tx).await;

                // Trigger screen refresh if hardware just connected and we have state
                if needs_refresh && app_state.needs_screen_refresh() {
                    if let Some(command) = build_display_command(&app_state) {
                        // Send always succeeds, overwrites any pending update with latest state
                        let _ = hardware_command_tx.send(Some(command));
                        app_state.mark_screen_update_completed();
                    }
                }
            }

            // Process Snapcast events
            Some(snap_event) = snapcast_event_rx.recv() => {
                let needs_refresh = handle_snapcast_event(&mut app_state, snap_event).await;

                // T052: Trigger screen refresh when RoomState changes
                if needs_refresh && app_state.needs_screen_refresh() {
                    // T054: Track time since state change for latency validation
                    if let Some(elapsed_ms) = app_state.time_since_state_change() {
                        println!("  Triggering screen refresh ({}ms since state change)", elapsed_ms);
                    } else {
                        println!("  Triggering screen refresh");
                    }

                    // Send display update command to hardware task
                    if let Some(command) = build_display_command(&app_state) {
                        // Send always succeeds, overwrites any pending update with latest state
                        let _ = hardware_command_tx.send(Some(command));

                        // Mark screen update as completed
                        app_state.mark_screen_update_completed();

                        // Validate latency
                        let (is_valid, elapsed) = app_state.validate_screen_update_latency();
                        if let Some(ms) = elapsed {
                            if is_valid {
                                println!("  Screen update latency: {}ms (within 2s limit)", ms);
                            }
                        }
                    }
                }
            }

            // T039: Process Home Assistant events
            Some(ha_event) = homeassistant_event_rx.recv() => {
                let needs_refresh = app_state.handle_homeassistant_event(ha_event);

                // Trigger screen refresh if on amplifier control page
                if needs_refresh && app_state.hardware_connected {
                    if let Some(command) = build_display_command(&app_state) {
                        let _ = hardware_command_tx.send(Some(command));
                        app_state.mark_screen_update_completed();
                    }
                }
            }

            // Run Snapcast message loop (non-blocking poll)
            _ = &mut snapcast_task => {
                eprintln!("Snapcast task ended unexpectedly");
            }

            // Handle Ctrl+C gracefully
            _ = tokio::signal::ctrl_c() => {
                println!("\nShutting down...");
                break;
            }
        }
    }

    // Clean shutdown
    hardware_monitor_task.abort();
    hardware_display_task.abort();

    println!("Application terminated");
}

/// Handle hardware events (T036, T055-T061)
async fn handle_hardware_event(
    state: &mut ApplicationState,
    event: HardwareEvent,
    _hardware_command_tx: &watch::Sender<Option<HardwareCommand>>,
    snapcast_command_tx: &tokio::sync::mpsc::UnboundedSender<SnapcastCommand>,
    homeassistant_command_tx: &tokio::sync::mpsc::UnboundedSender<HomeAssistantCommand>,
) -> bool {
    println!("Handle hardware event: {:?}", event);
    match event {
        HardwareEvent::DeviceConnected => {
            println!("Hardware event: Device connected");
            state.set_hardware_connected(true);

            // If server is already connected and we have room state, display it
            if state.server_connected && state.room.is_some() {
                println!("  Hardware connected, refreshing display");
                return true; // Trigger screen refresh
            } else {
                println!("  Waiting for server connection and room state...");
            }
        }
        HardwareEvent::DeviceDisconnected => {
            println!("Hardware event: Device disconnected");
            state.set_hardware_connected(false);
        }
        // T065-T066: Knob rotation controls volume (handled in ApplicationState)
        HardwareEvent::KnobRotated { knob_id, delta } => {
            // T055: Handle volume knob based on current page
            match state.current_page {
                crate::controller::state::PageView::AmplifierControl
                | crate::controller::state::PageView::SourceSelection => {
                    // T055: On amplifier pages, control amplifier volume via IR commands
                    if knob_id == 1 && state.homeassistant_connected {
                        // T060: Send appropriate IR command based on delta (positive = up, negative = down)
                        if delta > 0 {
                            for _ in 0..delta {
                                println!(
                                    "Hardware event: Knob {} rotated up - Amplifier Volume Up",
                                    knob_id
                                );
                                let _ = homeassistant_command_tx.send(HomeAssistantCommand::VolumeUp);
                            }
                        } else if delta < 0 {
                            for _ in 0..delta.abs() {
                                println!(
                                    "Hardware event: Knob {} rotated down - Amplifier Volume Down",
                                    knob_id
                                );
                                let _ = homeassistant_command_tx.send(HomeAssistantCommand::VolumeDown);
                            }
                        }
                        return false;
                    }
                }
                _ => {
                    // On other pages (Status, StreamSelection), control Snapcast volume
                    if let Some((client_id, new_volume)) = state.handle_knob_rotated(knob_id, delta) {
                        println!(
                            "Hardware event: Knob {} rotated (delta: {}) - Snapcast Volume: {}%",
                            knob_id, delta, new_volume
                        );

                        // T074: Mark control command sent for latency tracking
                        state.mark_control_command_sent();

                        // T066: Send SetVolume command to Snapcast
                        let _ = snapcast_command_tx.send(SnapcastCommand::SetVolume {
                            client_id,
                            volume: new_volume,
                        });

                        // State changed, trigger screen refresh
                        return false;
                    }
                }
            }
        }
        // T067-T068: Button press handling (depends on current page)
        HardwareEvent::ButtonPressed { button_id } => {
            match state.current_page {
                crate::controller::state::PageView::Status => {
                    // On status page, button 0 toggles mute
                    if let Some((client_id, new_muted)) = state.handle_button_pressed(button_id) {
                        println!(
                            "Hardware event: Button {} pressed - Mute: {}",
                            button_id, new_muted
                        );

                        // T074: Mark control command sent for latency tracking
                        state.mark_control_command_sent();

                        // T068: Send SetMuted command to Snapcast
                        let _ = snapcast_command_tx.send(SnapcastCommand::SetMuted {
                            client_id,
                            muted: new_muted,
                        });

                        // State changed, trigger screen refresh
                        return false;
                    }
                }
                crate::controller::state::PageView::StreamSelection => {
                    // T071: On stream selection page, buttons select streams
                    if button_id < 6 && (button_id as usize) < state.streams.len() {
                        println!(
                            "Hardware event: Button {} pressed - Selecting stream",
                            button_id
                        );
                        state.selected_stream_index = button_id as usize;

                        // T074: Mark control command sent for latency tracking
                        state.mark_control_command_sent();

                        // T072: Send AssignStream command
                        if let Some(room) = &state.room {
                            let stream_id = state.streams[button_id as usize].stream_id.clone();
                            let client_id = room.client_id.clone();
                            let _ = snapcast_command_tx.send(SnapcastCommand::SetStream {
                                client_id,
                                stream_id,
                            });
                        }

                        // Switch back to status page after selection
                        state.current_page = crate::controller::state::PageView::Status;
                        return true;
                    }
                }
                crate::controller::state::PageView::AmplifierControl => {
                    // T032: On amplifier control page, button 0 toggles power
                    if button_id == 0 {
                        // T036: Validate power toggle is allowed
                        if state.can_toggle_power() {
                            println!(
                                "Hardware event: Button {} pressed - Toggle amplifier power",
                                button_id
                            );

                            let _ =
                                homeassistant_command_tx.send(HomeAssistantCommand::TogglePower);

                            return false;
                        } else {
                            eprintln!("Cannot toggle power: not connected or state unknown");
                            return false;
                        }
                    }
                    // T051: Button 2 navigates to source selection page
                    else if button_id == 2 {
                        println!(
                            "Hardware event: Button {} pressed - Navigate to source selection",
                            button_id
                        );
                        state.current_page = crate::controller::state::PageView::SourceSelection;
                        return true;
                    }
                }
                crate::controller::state::PageView::SourceSelection => {
                    // T045: Buttons 0-4 select amplifier sources
                    if button_id <= 4 {
                        // T050: Validate source selection is allowed
                        if state.can_select_source() {
                            use crate::homeassistant::AmplifierSource;
                            let sources = [
                                AmplifierSource::Phono,
                                AmplifierSource::CD,
                                AmplifierSource::Spotify,
                                AmplifierSource::Source4,
                                AmplifierSource::Source5,
                            ];

                            if let Some(source) = sources.get(button_id as usize) {
                                println!(
                                    "Hardware event: Button {} pressed - Select source: {}",
                                    button_id,
                                    source.display_name()
                                );

                                // T047: Update local state
                                state.set_selected_source(*source);

                                // T048: Persist to file
                                if let Err(e) = state.save_selected_source() {
                                    eprintln!("Failed to save selected source: {}", e);
                                }

                                // T046: Send IR command via MQTT
                                let _ = homeassistant_command_tx.send(
                                    HomeAssistantCommand::SelectSource { source: *source },
                                );

                                // T051: Navigate back to amplifier control page
                                state.current_page =
                                    crate::controller::state::PageView::AmplifierControl;

                                return true;
                            }
                        } else {
                            // T052: Error handling
                            eprintln!("Cannot select source: Home Assistant not connected");
                            return false;
                        }
                    }
                }
                _ => {}
            }
        }
        // T070: Page button press handling for page switching
        HardwareEvent::PageButtonPressed { page_id } => {
            println!("Hardware event: Page button {} pressed", page_id);
            match page_id {
                0 => {
                    // Page button 0: Status page
                    state.current_page = crate::controller::state::PageView::Status;
                    return true;
                }
                1 => {
                    // Page button 1: Stream selection page
                    state.current_page = crate::controller::state::PageView::StreamSelection;
                    return true;
                }
                2 => {
                    // Page button 1: Stream selection page
                    state.current_page = crate::controller::state::PageView::AmplifierControl;
                    return true;
                }
                _ => {}
            }
        }
        _ => {
            // Other hardware events not yet implemented
        }
    }
    false
}

/// Handle Snapcast events (T037, T049-T052)
/// Returns true if screen refresh is needed
async fn handle_snapcast_event(state: &mut ApplicationState, event: SnapcastEvent) -> bool {
    println!("Handle snapcast event: {:?}", event);
    let changed = match event {
        SnapcastEvent::ServerReconnected { room, streams } => {
            println!("Snapcast event: Server reconnected");
            state.set_server_connected(true);

            // Update streams
            println!("  Received {} streams from server", streams.len());
            state.update_streams(streams);

            // Update room state
            if let Some(room_state) = room {
                println!(
                    "  Room '{}' found - Volume: {}%, Muted: {}, Connected: {}",
                    room_state.name, room_state.volume, room_state.muted, room_state.connected
                );
                state.update_room_state(room_state);

                // If hardware is connected, we could display the status now
                if state.hardware_connected {
                    println!("  Ready to display on hardware");
                    return true; // T052: Trigger screen refresh
                }
            } else {
                println!(
                    "  Warning: Room '{}' not found on server",
                    state.config.room.client_id
                );
            }
            false
        }
        SnapcastEvent::ServerDisconnected => {
            println!("Snapcast event: Server disconnected");
            state.set_server_connected(false);
            false
        }
        // T049: Handle volume/mute changes
        SnapcastEvent::ClientVolumeChanged {
            client_id,
            volume,
            muted,
        } => {
            println!(
                "Snapcast event: Volume changed for client '{}' - Volume: {}%, Muted: {}",
                client_id, volume, muted
            );

            // T074: Validate control command latency
            let (is_valid, elapsed) = state.validate_control_command_latency();
            if let Some(ms) = elapsed {
                if is_valid {
                    println!(
                        "  Control command feedback latency: {}ms (within 500ms limit)",
                        ms
                    );
                }
            }

            let changed = state.handle_volume_changed(&client_id, volume, muted);

            changed // T052: Return true if refresh needed
        }
        // T050: Handle stream changes
        SnapcastEvent::StreamChanged {
            group_id,
            stream_id,
        } => {
            println!(
                "Snapcast event: Stream changed for client '{}' to '{}'",
                group_id, stream_id
            );

            // T074: Validate control command latency
            let (is_valid, elapsed) = state.validate_control_command_latency();
            if let Some(ms) = elapsed {
                if is_valid {
                    println!(
                        "  Control command feedback latency: {}ms (within 500ms limit)",
                        ms
                    );
                }
            }

            let changed = state.handle_stream_changed(&group_id, stream_id);

            changed // T052: Return true if refresh needed
        }
        // T051: Handle stream updates
        SnapcastEvent::StreamUpdate { stream_id, stream } => {
            println!(
                "Snapcast event: Stream '{}' updated - Name: '{}', Status: {:?}",
                stream_id, stream.name, stream.status
            );
            let changed = state.handle_stream_update(&stream_id, stream);
            changed // T052: Return true if refresh needed
        }
        _ => {
            // Other Snapcast events (ClientConnected, ClientDisconnected)
            false
        }
    };
    eprintln!("Event changed: {}", changed);
    changed
}

/// Build StatusPageLayout from current application state
fn build_status_layout(state: &ApplicationState) -> Option<StatusPageLayout> {
    let room = state.room.as_ref()?;

    // Get stream name if we have a stream_id
    let stream_name = room
        .stream_id
        .as_ref()
        .and_then(|id| state.get_stream_name(id))
        .or(Some("No Stream".to_string()))?;

    Some(StatusPageLayout::from_room_state(
        &room.name,
        &format!(
            "{}:{}",
            state.config.server.address, state.config.server.port
        ),
        room.volume,
        room.muted,
        room.connected,
        Some(&stream_name),
    ))
}

/// Build appropriate HardwareCommand based on current page view
fn build_display_command(state: &ApplicationState) -> Option<HardwareCommand> {
    use crate::hardware::display::{SourceSelectionPageLayout, StreamSelectionPageLayout};

    match state.current_page {
        crate::controller::state::PageView::Status => {
            // Build and send status page layout
            let layout = build_status_layout(state)?;
            Some(HardwareCommand::UpdateStatusPage(layout))
        }
        crate::controller::state::PageView::StreamSelection => {
            // Build and send stream selection page layout
            let layout = StreamSelectionPageLayout::from_streams(
                &state.streams,
                state.selected_stream_index,
            );
            Some(HardwareCommand::UpdateStreamSelectionPage(layout))
        }
        crate::controller::state::PageView::AmplifierControl => {
            // Build and send amplifier control page layout
            let amplifier_state = state.get_amplifier_state()?;
            let selected_source = state
                .get_selected_source()
                .map(|s| s.display_name())
                .unwrap_or("Unknown");

            let layout = AmplifierControlPageLayout::new(
                amplifier_state.power_on,
                state.homeassistant_connected,
                selected_source,
                None, // No error for now
            );
            Some(HardwareCommand::UpdateAmplifierPage(layout))
        }
        crate::controller::state::PageView::SourceSelection => {
            // Build and send source selection page layout
            let selected_source = state.get_selected_source()?;
            let layout = SourceSelectionPageLayout::new(selected_source);
            Some(HardwareCommand::UpdateSourceSelectionPage(layout))
        }
        _ => {
            // Other pages not yet implemented
            None
        }
    }
}

/// Get the configuration file path
/// Checks in order:
/// 1. ./config.toml (current directory)
/// 2. ~/.config/snapcast-controller/config.toml (user config directory)
/// 3. /etc/snapcast-controller/config.toml (system config directory)
fn get_config_path() -> PathBuf {
    // Check current directory
    let local_config = PathBuf::from("config.toml");
    if local_config.exists() {
        return local_config;
    }

    // Check user config directory
    if let Some(home_dir) = dirs::home_dir() {
        let user_config = home_dir
            .join(".config")
            .join("snapcast-controller")
            .join("config.toml");
        if user_config.exists() {
            return user_config;
        }
    }

    // Check system config directory
    let system_config = PathBuf::from("/etc/snapcast-controller/config.toml");
    if system_config.exists() {
        return system_config;
    }

    // Default to local config.toml
    local_config
}

/// Load configuration from file
/// Exits the application if config cannot be loaded
fn load_config(path: &PathBuf) -> ConnectionSettings {
    match ConnectionSettings::load(path) {
        Ok(config) => config,
        Err(e) => {
            eprintln!("Failed to load configuration from {:?}", path);
            eprintln!("Error: {}", e);
            eprintln!();
            eprintln!("Please create a config.toml file with the following format:");
            eprintln!("[server]");
            eprintln!("address = \"192.168.1.100\"");
            eprintln!("port = 1705");
            eprintln!();
            eprintln!("[room]");
            eprintln!("client_id = \"living-room\"");
            std::process::exit(1);
        }
    }
}
