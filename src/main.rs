// Snapcast Controller Application
// Controls a single room's audio playback via USB HID hardware controller

use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Duration;

use tokio::sync::{mpsc::unbounded_channel, watch};
use tracing::{debug, error, info, warn};

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
mod models;
mod snapcast;

#[tokio::main]
async fn main() {
    // Initialize tracing subscriber for structured logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    info!("Snapcast Controller Application");
    info!("Initialization in progress...");

    // Load configuration (T024)
    let config_path = get_config_path();
    let config = load_config(&config_path);

    info!("Configuration loaded successfully");
    info!("  Server: {}:{}", config.server.address, config.server.port);
    info!("  Room client ID: {}", config.room.client_id);

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
                error!("Hardware monitor loop failed: {}", e);
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
                error!("Hardware display loop failed: {}", e);
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
            warn!("Failed to load amplifier state: {}", e);
        }

        // Load persisted unified source state
        if let Err(e) = app_state.load_unified_source_state() {
            warn!("Failed to load unified source state: {}", e);
        }

        Some(tokio::spawn(async move {
            mqtt_client.run(eventloop).await;
        }))
    } else {
        info!("Home Assistant not configured, skipping MQTT client");
        None
    };

    // T032: Implement main event loop
    info!("Starting main event loop...");

    // Pin the snapcast task so we can poll it
    tokio::pin!(snapcast_task);

    loop {
        tokio::select! {
            // Process hardware events
            Some(hw_event) = hardware_event_rx.recv() => {
                let needs_refresh = handle_hardware_event(&mut app_state, hw_event, &hardware_command_tx, &snapcast_command_tx, &homeassistant_command_tx).await;

                // Trigger screen refresh if hardware just connected and we have state
                if needs_refresh && app_state.needs_screen_refresh()
                    && let Some(command) = build_display_command(&app_state) {
                        // Send always succeeds, overwrites any pending update with latest state
                        let _ = hardware_command_tx.send(Some(command));
                        app_state.mark_screen_update_completed();
                    }
            }

            // Process Snapcast events
            Some(snap_event) = snapcast_event_rx.recv() => {
                let needs_refresh = handle_snapcast_event(&mut app_state, snap_event).await;

                // T052: Trigger screen refresh when RoomState changes
                if needs_refresh && app_state.needs_screen_refresh() {
                    // T054: Track time since state change for latency validation
                    if let Some(elapsed_ms) = app_state.time_since_state_change() {
                        debug!("Triggering screen refresh ({}ms since state change)", elapsed_ms);
                    } else {
                        debug!("Triggering screen refresh");
                    }

                    // Send display update command to hardware task
                    if let Some(command) = build_display_command(&app_state) {
                        // Send always succeeds, overwrites any pending update with latest state
                        let _ = hardware_command_tx.send(Some(command));

                        // Mark screen update as completed
                        app_state.mark_screen_update_completed();

                        // Validate latency
                        let (is_valid, elapsed) = app_state.validate_screen_update_latency();
                        if let Some(ms) = elapsed
                            && is_valid {
                                debug!("Screen update latency: {}ms (within 2s limit)", ms);
                            }
                    }
                }
            }

            // T039: Process Home Assistant events
            Some(ha_event) = homeassistant_event_rx.recv() => {
                let needs_refresh = app_state.handle_homeassistant_event(ha_event);

                // Trigger screen refresh if on amplifier control page
                if needs_refresh && app_state.hardware_connected
                    && let Some(command) = build_display_command(&app_state) {
                        let _ = hardware_command_tx.send(Some(command));
                        app_state.mark_screen_update_completed();
                    }
            }

            // Run Snapcast message loop (non-blocking poll)
            _ = &mut snapcast_task => {
                error!("Snapcast task ended unexpectedly");
            }

            // Handle Ctrl+C gracefully
            _ = tokio::signal::ctrl_c() => {
                info!("Shutting down...");
                break;
            }
        }
    }

    // Clean shutdown
    hardware_monitor_task.abort();
    hardware_display_task.abort();

    info!("Application terminated");
}

/// Handle hardware events (T036, T055-T061)
async fn handle_hardware_event(
    state: &mut ApplicationState,
    event: HardwareEvent,
    _hardware_command_tx: &watch::Sender<Option<HardwareCommand>>,
    snapcast_command_tx: &tokio::sync::mpsc::UnboundedSender<SnapcastCommand>,
    homeassistant_command_tx: &tokio::sync::mpsc::UnboundedSender<HomeAssistantCommand>,
) -> bool {
    debug!("Handle hardware event: {:?}", event);
    match event {
        HardwareEvent::DeviceConnected => {
            info!("Hardware device connected");
            state.set_hardware_connected(true);

            // If server is already connected and we have room state, display it
            if state.server_connected && state.room.is_some() {
                debug!("Hardware connected, refreshing display");
                return true; // Trigger screen refresh
            } else {
                debug!("Waiting for server connection and room state...");
            }
        }
        HardwareEvent::DeviceDisconnected => {
            info!("Hardware device disconnected");
            state.set_hardware_connected(false);
        }
        // T065-T066: Knob rotation controls volume (handled in ApplicationState)
        HardwareEvent::KnobRotated { knob_id, delta } => {
            // T055: Handle volume knob based on current page
            match state.current_page {
                crate::controller::state::PageView::AmplifierControl => {
                    // T055: On amplifier page, control amplifier volume via IR commands
                    if knob_id == 1 && state.homeassistant_connected {
                        // T060: Send appropriate IR command based on delta (positive = up, negative = down)
                        if delta > 0 {
                            for _ in 0..delta {
                                debug!("Knob {} rotated up - sending amplifier volume up", knob_id);
                                let _ =
                                    homeassistant_command_tx.send(HomeAssistantCommand::VolumeUp);
                            }
                        } else if delta < 0 {
                            for _ in 0..delta.abs() {
                                debug!(
                                    "Knob {} rotated down - sending amplifier volume down",
                                    knob_id
                                );
                                let _ =
                                    homeassistant_command_tx.send(HomeAssistantCommand::VolumeDown);
                            }
                        }
                        return false;
                    }
                }
                crate::controller::state::PageView::UnifiedSourceSelection => {
                    // On unified source selection page, knob navigates through sources
                    if knob_id == 1 {
                        if delta > 0 {
                            for _ in 0..delta {
                                debug!("Knob {} rotated up - navigating to next source", knob_id);
                                state.unified_source_select_next();
                            }
                        } else if delta < 0 {
                            for _ in 0..delta.abs() {
                                debug!("Knob {} rotated down - navigating to previous source", knob_id);
                                state.unified_source_select_previous();
                            }
                        }
                        // Trigger screen refresh to show new selection
                        return true;
                    }
                }
                _ => {
                    // On other pages (Status), control Snapcast volume
                    if let Some((client_id, new_volume)) = state.handle_knob_rotated(knob_id, delta)
                    {
                        debug!(
                            "Knob {} rotated (delta: {}) - setting Snapcast volume to {}%",
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
                        debug!(
                            "Button {} pressed - toggling mute to {}",
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
                crate::controller::state::PageView::AmplifierControl => {
                    // T032: On amplifier control page, button 0 toggles power
                    if button_id == 0 {
                        // T036: Validate power toggle is allowed
                        if state.can_toggle_power() {
                            debug!("Button {} pressed - toggling amplifier power", button_id);

                            let _ =
                                homeassistant_command_tx.send(HomeAssistantCommand::TogglePower);

                            return false;
                        } else {
                            warn!("Cannot toggle power: not connected or state unknown");
                            return false;
                        }
                    }
                }
                crate::controller::state::PageView::Settings => {
                    // Settings page not yet implemented
                }
                crate::controller::state::PageView::UnifiedSourceSelection => {
                    // Buttons 0-5 select unified sources (Snapcast streams or amplifier inputs)
                    // T048: Use unified_source_at_button to account for scrolling
                    if button_id < 6 {
                        if let Some(source) = state.unified_source_at_button(button_id) {
                        let source = source.clone();
                        info!(
                            "Button {} pressed - selecting unified source: {}",
                            button_id,
                            source.display_name()
                        );

                        // Get activation commands
                        let (amplifier_source, stream_id, client_id) =
                            state.activate_unified_source(&source);

                        // Save unified source state to file
                        if let Err(e) = state.save_unified_source_state() {
                            warn!("Failed to save unified source state: {}", e);
                        }

                        // Save amplifier source state to file (for AmplifierControl page)
                        if let Err(e) = state.save_selected_source() {
                            warn!("Failed to save amplifier source state: {}", e);
                        }

                        // Step 1: Switch amplifier input (always needed)
                        let _ = homeassistant_command_tx
                            .send(HomeAssistantCommand::SelectSource {
                                source: amplifier_source,
                            });

                        // Step 2: If this is a Snapcast stream, also send stream selection command
                        if let (Some(stream_id), Some(client_id)) = (stream_id, client_id) {
                            let _ = snapcast_command_tx.send(SnapcastCommand::SetStream {
                                client_id,
                                stream_id,
                            });
                        }

                        // Stay on unified source selection page to show the selection
                        // Screen will refresh automatically to show the updated selection
                        return true;
                        }
                    }
                }
            }
        }
        // T070: Page button press handling for page switching
        // T050: Updated to include UnifiedSourceSelection page
        HardwareEvent::PageButtonPressed { page_id } => {
            debug!("Page button {} pressed", page_id);
            match page_id {
                0 => {
                    // Page button 0: Status page
                    state.current_page = crate::controller::state::PageView::Status;
                    return true;
                }
                1 => {
                    // Page button 1: Unified source selection page (Snapcast streams + amplifier inputs)
                    state.current_page = crate::controller::state::PageView::UnifiedSourceSelection;
                    return true;
                }
                2 => {
                    // Page button 2: Amplifier control page
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
    debug!("Handling Snapcast event: {:?}", event);
    let changed = match event {
        SnapcastEvent::ServerReconnected { room, streams } => {
            info!("Snapcast server reconnected");
            state.set_server_connected(true);

            // Update streams
            info!("Received {} streams from Snapcast server", streams.len());
            state.update_streams(streams);

            // Update room state
            if let Some(room_state) = room {
                info!(
                    "Room '{}' found - Volume: {}%, Muted: {}, Connected: {}",
                    room_state.name, room_state.volume, room_state.muted, room_state.connected
                );
                state.update_room_state(room_state);

                // If hardware is connected, we could display the status now
                if state.hardware_connected {
                    debug!("Hardware ready - triggering display refresh");
                    return true; // T052: Trigger screen refresh
                }
            } else {
                warn!(
                    "Room '{}' not found on Snapcast server",
                    state.config.room.client_id
                );
            }
            false
        }
        SnapcastEvent::ServerDisconnected => {
            warn!("Snapcast server disconnected");
            state.set_server_connected(false);
            false
        }
        // T049: Handle volume/mute changes
        SnapcastEvent::ClientVolumeChanged {
            client_id,
            volume,
            muted,
        } => {
            debug!(
                "Client '{}' volume changed - Volume: {}%, Muted: {}",
                client_id, volume, muted
            );

            // T074: Validate control command latency
            let (is_valid, elapsed) = state.validate_control_command_latency();
            if let Some(ms) = elapsed
                && is_valid
            {
                debug!(
                    "Control command feedback latency: {}ms (within 500ms limit)",
                    ms
                );
            }

            state.handle_volume_changed(&client_id, volume, muted) // T052: Return true if refresh needed
        }
        // T050: Handle stream changes
        SnapcastEvent::StreamChanged {
            group_id,
            stream_id,
        } => {
            info!("Stream changed for group '{}' to '{}'", group_id, stream_id);

            // T074: Validate control command latency
            let (is_valid, elapsed) = state.validate_control_command_latency();
            if let Some(ms) = elapsed
                && is_valid
            {
                debug!(
                    "Control command feedback latency: {}ms (within 500ms limit)",
                    ms
                );
            }

            state.handle_stream_changed(&group_id, stream_id) // T052: Return true if refresh needed
        }
        // T051: Handle stream updates
        SnapcastEvent::StreamUpdate { stream_id, stream } => {
            debug!(
                "Stream '{}' updated - Name: '{}', Status: {:?}",
                stream_id, stream.name, stream.status
            );

            state.handle_stream_update(&stream_id, stream) // T052: Return true if refresh needed
        }
        _ => {
            // Other Snapcast events (ClientConnected, ClientDisconnected)
            false
        }
    };
    debug!("Event state changed: {}", changed);
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
    use crate::hardware::display::{
        UnifiedSourceSelectionPageLayout,
    };

    match state.current_page {
        crate::controller::state::PageView::Status => {
            // Build and send status page layout
            let layout = build_status_layout(state)?;
            Some(HardwareCommand::UpdateStatusPage(layout))
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
        crate::controller::state::PageView::UnifiedSourceSelection => {
            // Build and send unified source selection page layout
            let sources = state.unified_sources_all();

            // Highlight the currently ACTIVE source (what's playing now)
            // not just the navigation cursor position
            let selected_index = state
                .unified_source_active()
                .and_then(|active| {
                    sources
                        .iter()
                        .position(|s| s.identifier() == active.identifier())
                })
                .unwrap_or(0);

            let layout = UnifiedSourceSelectionPageLayout::from_sources(sources, selected_index);
            Some(HardwareCommand::UpdateUnifiedSourceSelectionPage(layout))
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
            error!("Failed to load configuration from {:?}", path);
            error!("Error: {}", e);
            error!("");
            error!("Please create a config.toml file with the following format:");
            error!("[server]");
            error!("address = \"192.168.1.100\"");
            error!("port = 1705");
            error!("");
            error!("[room]");
            error!("client_id = \"living-room\"");
            std::process::exit(1);
        }
    }
}
