// Snapcast Controller Application
// Controls a single room's audio playback via USB HID hardware controller

use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Duration;

use tokio::sync::mpsc::unbounded_channel;

use crate::config::settings::ConnectionSettings;
use crate::controller::state::ApplicationState;
use crate::hardware::{device::DeviceManager, events::{HardwareEvent, HardwareCommand}, display::StatusPageLayout};
use crate::snapcast::{client::SnapcastClient, types::{SnapcastEvent, SnapcastCommand}};

mod config;
mod controller;
mod hardware;
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
    let (hardware_command_tx, hardware_command_rx) = unbounded_channel::<HardwareCommand>();
    let (snapcast_command_tx, snapcast_command_rx) = unbounded_channel::<SnapcastCommand>();

    // Parse server address
    let server_addr = format!("{}:{}", config.server.address, config.server.port)
        .parse::<SocketAddr>()
        .expect("Invalid server address");

    // T033: Create tokio task for hardware event listening and display updates
    let hardware_manager = DeviceManager::new(hardware_event_tx);
    let hardware_task = tokio::spawn(async move {
        if let Err(e) = hardware::device::device_monitor_loop(
            hardware_manager,
            Duration::from_millis(10),
            hardware_command_rx,
        )
        .await {
            eprintln!("Hardware monitor loop failed: {}", e);
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
        let _ = snapcast::client::message_loop(snapcast_client, Duration::from_secs(2), snapcast_command_rx).await;
    };

    // T032: Implement main event loop
    println!("Starting main event loop...");

    // Pin the snapcast task so we can poll it
    tokio::pin!(snapcast_task);

    loop {
        tokio::select! {
            // Process hardware events
            Some(hw_event) = hardware_event_rx.recv() => {
                let needs_refresh = handle_hardware_event(&mut app_state, hw_event, &hardware_command_tx, &snapcast_command_tx).await;

                // Trigger screen refresh if hardware just connected and we have state
                if needs_refresh && app_state.needs_screen_refresh() {
                    if let Some(layout) = build_status_layout(&app_state) {
                        let _ = hardware_command_tx.send(HardwareCommand::UpdateStatusPage(layout));
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
                    if let Some(layout) = build_status_layout(&app_state) {
                        let _ = hardware_command_tx.send(HardwareCommand::UpdateStatusPage(layout));

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
    hardware_task.abort();

    println!("Application terminated");
}

/// Handle hardware events (T036, T055-T061)
async fn handle_hardware_event(
    state: &mut ApplicationState,
    event: HardwareEvent,
    _hardware_command_tx: &tokio::sync::mpsc::UnboundedSender<HardwareCommand>,
    snapcast_command_tx: &tokio::sync::mpsc::UnboundedSender<SnapcastCommand>,
) -> bool {
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
            if let Some((client_id, new_volume)) = state.handle_knob_rotated(knob_id, delta) {
                println!("Hardware event: Knob {} rotated (delta: {}) - Volume: {}%",
                         knob_id, delta, new_volume);

                // T066: Send SetVolume command to Snapcast
                let _ = snapcast_command_tx.send(SnapcastCommand::SetVolume {
                    client_id,
                    volume: new_volume,
                });
            }
        }
        // T067-T068: Button press handling (handled in ApplicationState)
        HardwareEvent::ButtonPressed { button_id } => {
            if let Some((client_id, new_muted)) = state.handle_button_pressed(button_id) {
                println!("Hardware event: Button {} pressed - Mute: {}",
                         button_id, new_muted);

                // T068: Send SetMuted command to Snapcast
                let _ = snapcast_command_tx.send(SnapcastCommand::SetMuted {
                    client_id,
                    muted: new_muted,
                });
            } else if button_id == 1 {
                // T061: Button 1 switches to stream selection page
                println!("Hardware event: Button 1 pressed - Switching to stream selection");
                state.current_page = crate::controller::state::PageView::StreamSelection;
                // TODO: Implement stream selection page rendering in future phase
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
    match event {
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
                    room_state.name,
                    room_state.volume,
                    room_state.muted,
                    room_state.connected
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
        SnapcastEvent::ClientVolumeChanged { client_id, volume, muted } => {
            println!("Snapcast event: Volume changed for client '{}' - Volume: {}%, Muted: {}",
                     client_id, volume, muted);
            let changed = state.handle_volume_changed(&client_id, volume, muted);
            if changed {
                println!("  Room state updated, triggering screen refresh");
            }
            changed // T052: Return true if refresh needed
        }
        // T050: Handle stream changes
        SnapcastEvent::StreamChanged { client_id, stream_id } => {
            println!("Snapcast event: Stream changed for client '{}' to '{}'",
                     client_id, stream_id);
            let changed = state.handle_stream_changed(&client_id, stream_id);
            if changed {
                println!("  Room state updated, triggering screen refresh");
            }
            changed // T052: Return true if refresh needed
        }
        // T051: Handle stream updates
        SnapcastEvent::StreamUpdate { stream_id, stream } => {
            println!("Snapcast event: Stream '{}' updated - Name: '{}', Status: {:?}",
                     stream_id, stream.name, stream.status);
            let changed = state.handle_stream_update(&stream_id, stream);
            if changed {
                println!("  Stream state updated, triggering screen refresh");
            }
            changed // T052: Return true if refresh needed
        }
        _ => {
            // Other Snapcast events (ClientConnected, ClientDisconnected)
            false
        }
    }
}

/// Build StatusPageLayout from current application state
fn build_status_layout(state: &ApplicationState) -> Option<StatusPageLayout> {
    let room = state.room.as_ref()?;

    // Get stream name if we have a stream_id
    let stream_name = room.stream_id.as_ref()
        .and_then(|id| state.get_stream_name(id))
        .or(Some("No Stream".to_string()))?;

    let layout = StatusPageLayout::from_room_state(
        &room.name,
        &format!("{}:{}", state.config.server.address, state.config.server.port),
        room.volume,
        room.muted,
        room.connected,
        Some(&stream_name),
    );

    Some(layout)
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
