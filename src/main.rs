// Snapcast Controller Application
// Controls a single room's audio playback via USB HID hardware controller

use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Duration;

use tokio::sync::mpsc::unbounded_channel;

use crate::config::settings::ConnectionSettings;
use crate::controller::state::ApplicationState;
use crate::hardware::{device::DeviceManager, events::HardwareEvent};
use crate::snapcast::{client::SnapcastClient, types::SnapcastEvent};

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

    // Parse server address
    let server_addr = format!("{}:{}", config.server.address, config.server.port)
        .parse::<SocketAddr>()
        .expect("Invalid server address");

    // T033: Create tokio task for hardware event listening
    let hardware_manager = DeviceManager::new(hardware_event_tx);
    let hardware_task = tokio::spawn(async move {
        let _ = hardware::device::device_monitor_loop(
            hardware_manager,
            Duration::from_millis(1000),
        )
        .await;
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
        let _ = snapcast::client::message_loop(snapcast_client, Duration::from_secs(2)).await;
    };

    // T032: Implement main event loop
    println!("Starting main event loop...");

    // Pin the snapcast task so we can poll it
    tokio::pin!(snapcast_task);

    loop {
        tokio::select! {
            // Process hardware events
            Some(hw_event) = hardware_event_rx.recv() => {
                handle_hardware_event(&mut app_state, hw_event).await;
            }

            // Process Snapcast events
            Some(snap_event) = snapcast_event_rx.recv() => {
                handle_snapcast_event(&mut app_state, snap_event).await;
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

/// Handle hardware events (T036)
async fn handle_hardware_event(state: &mut ApplicationState, event: HardwareEvent) {
    match event {
        HardwareEvent::DeviceConnected => {
            println!("Hardware event: Device connected");
            state.set_hardware_connected(true);

            // If server is already connected and we have room state, display it
            if state.server_connected {
                if let Some(room) = &state.room {
                    println!(
                        "  Displaying room '{}' status on hardware",
                        room.name
                    );
                } else {
                    println!("  Waiting for room state from server...");
                }
            } else {
                println!("  Waiting for server connection...");
            }
        }
        HardwareEvent::DeviceDisconnected => {
            println!("Hardware event: Device disconnected");
            state.set_hardware_connected(false);
        }
        _ => {
            // Other hardware events (knobs, buttons) will be handled in future phases
        }
    }
}

/// Handle Snapcast events (T037)
async fn handle_snapcast_event(state: &mut ApplicationState, event: SnapcastEvent) {
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
                }
            } else {
                println!(
                    "  Warning: Room '{}' not found on server",
                    state.config.room.client_id
                );
            }
        }
        SnapcastEvent::ServerDisconnected => {
            println!("Snapcast event: Server disconnected");
            state.set_server_connected(false);
        }
        _ => {
            // Other Snapcast events will be handled in future phases
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
