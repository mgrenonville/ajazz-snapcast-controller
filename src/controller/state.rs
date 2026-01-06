// Controller state - Central state machine for the application

use crate::config::settings::ConnectionSettings;
use crate::homeassistant::AmplifierSource;
use crate::homeassistant::types::{AmplifierState, HomeAssistantEvent};
use crate::models::UnifiedSourceCollection;
use crate::snapcast::types::{AudioStream, RoomState};
use std::fs;
use std::path::PathBuf;
use std::time::Instant;
use tracing::{debug, error, info, warn};

/// Central state machine for the application
#[derive(Debug)]
pub struct ApplicationState {
    /// Loaded configuration from file
    pub config: ConnectionSettings,

    /// Current room state (None if server disconnected)
    pub room: Option<RoomState>,

    /// Available streams on server
    pub streams: Vec<AudioStream>,

    /// USB controller connection status
    pub hardware_connected: bool,

    /// Snapcast server connection status
    pub server_connected: bool,

    /// Home Assistant MQTT broker connection status
    pub homeassistant_connected: bool,

    /// Amplifier state (None if Home Assistant not configured or disconnected)
    pub amplifier: Option<AmplifierState>,

    /// Unified source collection (Snapcast streams + amplifier inputs)
    pub unified_sources: UnifiedSourceCollection,

    /// UI state for stream selection (index into streams vec)
    pub selected_stream_index: usize,

    /// Which page is currently displayed on hardware
    pub current_page: PageView,

    /// T054: Timestamp of last state change that requires screen refresh
    last_state_change: Option<Instant>,

    /// T054: Timestamp of last screen update completion
    last_screen_update: Option<Instant>,

    /// T074: Timestamp of last control command sent (volume/mute/stream change)
    last_control_command: Option<Instant>,
}

/// Page views displayed on hardware controller
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageView {
    /// Show room volume, mute status, current stream
    Status,

    /// Show connection status, server address
    Settings,

    /// Show amplifier power and source control
    AmplifierControl,

    /// Show unified source selection (Snapcast streams + amplifier inputs)
    /// Replaces deprecated StreamSelection and SourceSelection pages (T051, T052)
    UnifiedSourceSelection,
}

impl ApplicationState {
    /// Create a new ApplicationState with loaded configuration
    pub fn new(config: ConnectionSettings) -> Self {
        // Initialize amplifier state if Home Assistant is configured
        let amplifier = if config.homeassistant.is_some() {
            Some(AmplifierState::new())
        } else {
            None
        };

        // Determine Snapcast amplifier source from config
        let snapcast_amplifier_source = config
            .homeassistant
            .as_ref()
            .map(|ha| ha.amplifier.snapcast_amplifier_source)
            .unwrap_or(AmplifierSource::Spotify);

        // Initialize unified source collection (will be populated on server connection)
        let unified_sources = UnifiedSourceCollection::new(Vec::new(), snapcast_amplifier_source);

        Self {
            config,
            room: None,
            streams: Vec::new(),
            hardware_connected: false,
            server_connected: false,
            homeassistant_connected: false,
            amplifier,
            unified_sources,
            selected_stream_index: 0,
            current_page: PageView::Status,
            last_state_change: None,
            last_screen_update: None,
            last_control_command: None,
        }
    }

    /// Update hardware connection status
    pub fn set_hardware_connected(&mut self, connected: bool) {
        self.hardware_connected = connected;
    }

    /// Update server connection status
    pub fn set_server_connected(&mut self, connected: bool) {
        self.server_connected = connected;
        if !connected {
            // Clear room state when server disconnects
            self.room = None;
            self.streams.clear();
        }
    }

    /// Update room state from server
    pub fn update_room_state(&mut self, room: RoomState) {
        self.room = Some(room);
    }

    /// Update available streams
    pub fn update_streams(&mut self, streams: Vec<AudioStream>) {
        self.streams = streams.clone();

        // Update unified source collection with new Snapcast streams
        self.unified_sources.update_snapcast_streams(streams);
    }

    /// T049: Handle ClientVolumeChanged event
    /// Returns true if state changed and screen refresh is needed
    pub fn handle_volume_changed(&mut self, client_id: &str, volume: u8, muted: bool) -> bool {
        // Only update if this is our room's client
        if let Some(room) = &mut self.room
            && room.client_id == client_id
        {
            let changed = room.volume != volume || room.muted != muted;
            if changed {
                room.volume = volume;
                room.muted = muted;
                // T054: Mark state change timestamp
                self.last_state_change = Some(Instant::now());
                return true;
            }
        }
        false
    }

    /// T050: Handle StreamChanged event
    /// Returns true if state changed and screen refresh is needed
    pub fn handle_stream_changed(&mut self, group_id: &str, stream_id: String) -> bool {
        // Only update if this is our room's client
        if let Some(room) = &mut self.room
            && room.group_id == group_id
        {
            let changed = room.stream_id.as_ref() != Some(&stream_id);
            if changed {
                room.stream_id = Some(stream_id);
                // T054: Mark state change timestamp
                self.last_state_change = Some(Instant::now());
                return true;
            }
        }
        false
    }

    /// T051: Handle Stream.OnUpdate notification
    /// Updates stream metadata in the streams list
    /// Returns true if state changed and screen refresh is needed
    pub fn handle_stream_update(&mut self, stream_id: &str, updated_stream: AudioStream) -> bool {
        // Find and update the stream in our list
        if let Some(stream) = self.streams.iter_mut().find(|s| s.stream_id == stream_id) {
            // Check if anything actually changed
            let changed = stream.name != updated_stream.name
                || stream.status != updated_stream.status
                || format!("{:?}", stream.metadata) != format!("{:?}", updated_stream.metadata);

            if changed {
                *stream = updated_stream;
                // Only trigger refresh if this is the currently playing stream
                if let Some(room) = &self.room
                    && room.stream_id.as_ref() == Some(&stream_id.to_string())
                {
                    // T054: Mark state change timestamp
                    self.last_state_change = Some(Instant::now());
                    return true;
                }
            }
        }
        false
    }

    /// T052: Check if screen refresh is needed
    /// This is called after state updates to determine if we should redraw screens
    pub fn needs_screen_refresh(&self) -> bool {
        // We need both hardware connected and a valid room state to display
        self.hardware_connected && self.room.is_some()
    }

    /// Get stream name by stream ID
    pub fn get_stream_name(&self, stream_id: &str) -> Option<String> {
        self.streams
            .iter()
            .find(|s| s.stream_id == stream_id)
            .map(|s| s.name.clone())
    }

    /// T054: Mark screen update as completed
    pub fn mark_screen_update_completed(&mut self) {
        self.last_screen_update = Some(Instant::now());
    }

    /// T054: Validate that screen update happened within 2 seconds of state change
    /// Returns (is_valid, elapsed_time_ms)
    pub fn validate_screen_update_latency(&self) -> (bool, Option<u128>) {
        if let (Some(state_change), Some(screen_update)) =
            (self.last_state_change, self.last_screen_update)
        {
            // Screen update should happen after state change
            if screen_update >= state_change {
                let elapsed = screen_update.duration_since(state_change);
                let elapsed_ms = elapsed.as_millis();

                // Validate < 2 seconds (2000ms)
                let is_valid = elapsed_ms < 2000;

                if !is_valid {
                    warn!("Screen update latency exceeded 2 seconds: {}ms", elapsed_ms);
                }

                return (is_valid, Some(elapsed_ms));
            }
        }

        // No timestamps available yet
        (true, None)
    }

    /// T054: Get time since last state change
    pub fn time_since_state_change(&self) -> Option<u128> {
        self.last_state_change.map(|t| t.elapsed().as_millis())
    }

    /// T074: Mark that a control command was sent
    pub fn mark_control_command_sent(&mut self) {
        self.last_control_command = Some(Instant::now());
    }

    /// T074: Validate control command feedback latency (< 500ms)
    /// Should be called when server confirms command execution
    /// Returns (is_valid, elapsed_time_ms)
    pub fn validate_control_command_latency(&mut self) -> (bool, Option<u128>) {
        if let Some(command_time) = self.last_control_command {
            let elapsed = command_time.elapsed();
            let elapsed_ms = elapsed.as_millis();

            // Validate < 500ms
            let is_valid = elapsed_ms < 500;

            if !is_valid {
                warn!(
                    "Control command feedback latency exceeded 500ms: {}ms",
                    elapsed_ms
                );
            }

            // Clear the timestamp after validation
            self.last_control_command = None;

            return (is_valid, Some(elapsed_ms));
        }

        // No command timestamp available
        (true, None)
    }

    /// T065: Handle knob rotation events
    pub fn handle_knob_rotated(&self, knob_id: u8, delta: i8) -> Option<(String, u8)> {
        // Only knob 0 controls volume
        if knob_id != 1 {
            return None;
        }

        // Get current room state
        let room = self.room.as_ref()?;

        // T058 & T059: Calculate new volume (delta * 5%), clamped to 0-100
        let volume_change = delta as i32 * 5;
        let new_volume = (room.volume as i32 + volume_change).clamp(0, 100) as u8;

        // Only return if volume actually changed
        if new_volume != room.volume {
            let client_id = room.client_id.clone();

            Some((client_id, new_volume))
        } else {
            None
        }
    }

    /// T067: Handle button press events for mute toggle
    /// Updates local state optimistically and returns Some(new_muted_state) if button 0 was pressed
    pub fn handle_button_pressed(&self, button_id: u8) -> Option<(String, bool)> {
        // Only button 0 toggles mute
        if button_id != 0 {
            return None;
        }

        // Get current room state
        let room = self.room.as_ref()?;

        // Toggle muted state
        let new_muted = !room.muted;
        let client_id = room.client_id.clone();

        Some((client_id, new_muted))
    }

    /// Handle Home Assistant events and update amplifier state
    /// Returns true if screen refresh is needed
    pub fn handle_homeassistant_event(&mut self, event: HomeAssistantEvent) -> bool {
        match event {
            HomeAssistantEvent::BrokerConnected => {
                self.homeassistant_connected = true;
                info!("Home Assistant connected");
                // Refresh screen if on amplifier control page
                self.current_page == PageView::AmplifierControl
            }

            HomeAssistantEvent::BrokerDisconnected => {
                self.homeassistant_connected = false;
                warn!("Home Assistant disconnected");
                // Mark amplifier power as unknown (keep selected_source as it's tracked locally)
                if let Some(ref mut amplifier) = self.amplifier {
                    amplifier.power_on = None;
                    amplifier
                        .set_availability(crate::homeassistant::types::EntityAvailability::Unknown);
                }
                // Refresh screen if on amplifier control page
                self.current_page == PageView::AmplifierControl
            }

            HomeAssistantEvent::EntityStateChanged { entity_id, state } => {
                if let Some(ref mut amplifier) = self.amplifier {
                    // Only handle power state changes (source is managed locally)
                    if let Some(ref ha_config) = self.config.homeassistant
                        && entity_id == ha_config.amplifier.power_entity
                    {
                        // Power state changed
                        let power_on = state.to_uppercase() == "ON";
                        amplifier.set_power(power_on);
                        info!("Amplifier power: {}", if power_on { "ON" } else { "OFF" });
                        self.last_state_change = Some(Instant::now());
                        return self.current_page == PageView::AmplifierControl;
                    }
                }
                false
            }

            HomeAssistantEvent::EntityAvailabilityChanged {
                entity_id: _,
                available,
            } => {
                if let Some(ref mut amplifier) = self.amplifier {
                    let availability = if available {
                        crate::homeassistant::types::EntityAvailability::Available
                    } else {
                        crate::homeassistant::types::EntityAvailability::Unavailable
                    };
                    amplifier.set_availability(availability);
                    debug!("Amplifier availability: {:?}", availability);
                    self.last_state_change = Some(Instant::now());
                    return self.current_page == PageView::AmplifierControl;
                }
                false
            }

            HomeAssistantEvent::CommandAcknowledged { entity_id } => {
                debug!("Command acknowledged for {}", entity_id);
                // Could show visual feedback here
                false
            }

            HomeAssistantEvent::CommandFailed { entity_id, error } => {
                error!("Command failed for {}: {}", entity_id, error);
                // Could show error on screen
                self.current_page == PageView::AmplifierControl
            }
        }
    }

    /// Get amplifier state reference
    pub fn get_amplifier_state(&self) -> Option<&AmplifierState> {
        self.amplifier.as_ref()
    }

    /// Check if power toggle is allowed
    pub fn can_toggle_power(&self) -> bool {
        self.homeassistant_connected
            && self
                .amplifier
                .as_ref()
                .map(|a| a.power_on.is_some())
                .unwrap_or(false)
    }

    /// Check if source selection is allowed
    pub fn can_select_source(&self) -> bool {
        self.homeassistant_connected && self.amplifier.is_some()
    }

    /// Update selected source (local state)
    pub fn set_selected_source(&mut self, source: crate::homeassistant::AmplifierSource) {
        if let Some(ref mut amplifier) = self.amplifier {
            amplifier.set_source(source);
            self.last_state_change = Some(Instant::now());
        }
    }

    /// Get selected source
    pub fn get_selected_source(&self) -> Option<AmplifierSource> {
        self.amplifier.as_ref().map(|a| a.selected_source)
    }

    /// Get path to amplifier state file
    fn get_amplifier_state_file() -> Option<PathBuf> {
        dirs::config_dir().map(|mut path| {
            path.push("snapcast-controller");
            path.push("amplifier_state.json");
            path
        })
    }

    /// Save selected source to file
    pub fn save_selected_source(&self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(source) = self.get_selected_source()
            && let Some(state_file) = Self::get_amplifier_state_file()
        {
            // Create parent directory if it doesn't exist
            if let Some(parent) = state_file.parent() {
                fs::create_dir_all(parent)?;
            }

            // Serialize and save
            let json = serde_json::to_string(&source)?;
            fs::write(state_file, json)?;
        }
        Ok(())
    }

    /// Load selected source from file
    pub fn load_selected_source(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(state_file) = Self::get_amplifier_state_file()
            && state_file.exists()
        {
            let json = fs::read_to_string(state_file)?;
            let source: AmplifierSource = serde_json::from_str(&json)?;

            // Set the source in amplifier state
            self.set_selected_source(source);

            info!("Loaded amplifier source: {}", source.display_name());
        }
        Ok(())
    }

    /// T063: Navigate to next page in the main page cycle
    /// Cycle: Status → UnifiedSourceSelection → AmplifierControl → Status
    /// Legacy: StreamSelection and SourceSelection are sub-pages
    pub fn next_page(&mut self) {
        self.current_page = match self.current_page {
            PageView::Status => PageView::UnifiedSourceSelection,
            PageView::UnifiedSourceSelection => {
                // Only go to AmplifierControl if Home Assistant is configured
                if self.amplifier.is_some() {
                    PageView::AmplifierControl
                } else {
                    PageView::Status
                }
            }
            PageView::AmplifierControl => PageView::Status,
            PageView::Settings => PageView::Status, // Settings not implemented yet
        };
    }

    /// T064: Navigate to previous page in the main page cycle
    /// Cycle: Status ← AmplifierControl ← UnifiedSourceSelection ← Status
    pub fn previous_page(&mut self) {
        self.current_page = match self.current_page {
            PageView::Status => {
                // Only go to AmplifierControl if Home Assistant is configured
                if self.amplifier.is_some() {
                    PageView::AmplifierControl
                } else {
                    PageView::UnifiedSourceSelection
                }
            }
            PageView::UnifiedSourceSelection => PageView::Status,
            PageView::AmplifierControl => PageView::UnifiedSourceSelection,
            PageView::Settings => PageView::Status, // Settings not implemented yet
        };
    }

    /// Navigate unified source selection up (wraps around)
    pub fn unified_source_select_previous(&mut self) {
        self.unified_sources.select_previous();
        self.last_state_change = Some(Instant::now());
    }

    /// Navigate unified source selection down (wraps around)
    pub fn unified_source_select_next(&mut self) {
        self.unified_sources.select_next();
        self.last_state_change = Some(Instant::now());
    }

    /// Get currently selected unified source (for UI navigation)
    pub fn unified_source_selected(&self) -> Option<&crate::models::UnifiedSource> {
        self.unified_sources.selected_source()
    }

    /// Get all available unified sources
    pub fn unified_sources_all(&self) -> &[crate::models::UnifiedSource] {
        self.unified_sources.all_sources()
    }

    /// Get currently active unified source
    pub fn unified_source_active(&self) -> Option<&crate::models::UnifiedSource> {
        self.unified_sources.active_source()
    }

    /// Get unified source at a specific button position, accounting for scrolling (T048)
    /// Returns the source that should be activated when a button is pressed
    pub fn unified_source_at_button(&self, button_id: u8) -> Option<&crate::models::UnifiedSource> {
        let sources = self.unified_sources.all_sources();
        let total_sources = sources.len();

        if total_sources == 0 {
            return None;
        }

        // Calculate the same window_start logic as in the layout
        let selected_index = self
            .unified_source_selected()
            .and_then(|selected| {
                sources
                    .iter()
                    .position(|s| s.identifier() == selected.identifier())
            })
            .unwrap_or(0);

        let window_start = if total_sources <= 6 {
            0
        } else if selected_index < 3 {
            0
        } else if selected_index >= total_sources - 3 {
            total_sources.saturating_sub(6)
        } else {
            selected_index.saturating_sub(2)
        };

        // Map button to source index
        let source_index = window_start + (button_id as usize);
        sources.get(source_index)
    }

    /// Activate a unified source (returns commands to execute)
    /// Returns: (amplifier_source, optional_stream_id, optional_client_id)
    pub fn activate_unified_source(
        &mut self,
        source: &crate::models::UnifiedSource,
    ) -> (AmplifierSource, Option<String>, Option<String>) {
        // Get Snapcast amplifier source from config
        let snapcast_amplifier_source = self
            .config
            .homeassistant
            .as_ref()
            .map(|ha| ha.amplifier.snapcast_amplifier_source)
            .unwrap_or(AmplifierSource::Spotify);

        // Create activation context
        let context =
            crate::models::SourceActivationContext::new(source, snapcast_amplifier_source);

        // Get activation commands
        let (amplifier_source, stream_id) = context.get_activation_commands();

        // Get client ID if we're switching to a Snapcast stream
        let client_id = if stream_id.is_some() {
            self.room.as_ref().map(|r| r.client_id.clone())
        } else {
            None
        };

        // Update active source in collection
        self.unified_sources.set_active(source.clone());

        // Also update the AmplifierState.selected_source for backward compatibility
        // with AmplifierControl page display
        self.set_selected_source(amplifier_source);

        // Mark state change
        self.last_state_change = Some(Instant::now());

        info!("Activating unified source: {} -> amplifier input {}",
              source.display_name(), amplifier_source.display_name());

        (amplifier_source, stream_id, client_id)
    }

    /// Save unified source state to file
    pub fn save_unified_source_state(&self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(active_source) = self.unified_source_active() {
            let state = crate::models::UnifiedSourceState::new(active_source.clone());
            state.save()?;
            debug!("Saved unified source state: {}", active_source.display_name());
        }
        Ok(())
    }

    /// Load unified source state from file
    pub fn load_unified_source_state(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(state) = crate::models::UnifiedSourceState::load()? {
            // Set the active source in the collection
            self.unified_sources.set_active(state.active_source.clone());

            // Also restore the AmplifierState.selected_source for backward compatibility
            // Determine which amplifier input should be active based on the unified source
            let snapcast_amplifier_source = self
                .config
                .homeassistant
                .as_ref()
                .map(|ha| ha.amplifier.snapcast_amplifier_source)
                .unwrap_or(AmplifierSource::Spotify);

            let context = crate::models::SourceActivationContext::new(
                &state.active_source,
                snapcast_amplifier_source,
            );
            let (amplifier_source, _) = context.get_activation_commands();
            self.set_selected_source(amplifier_source);

            info!(
                "Loaded unified source state: {} -> amplifier input {}",
                state.active_source.display_name(),
                amplifier_source.display_name()
            );
        }
        Ok(())
    }
}
