// Controller state - Central state machine for the application

use crate::config::settings::ConnectionSettings;
use crate::snapcast::types::{AudioStream, RoomState};
use std::time::Instant;

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

    /// UI state for stream selection (index into streams vec)
    pub selected_stream_index: usize,

    /// Which page is currently displayed on hardware
    pub current_page: PageView,

    /// T054: Timestamp of last state change that requires screen refresh
    last_state_change: Option<Instant>,

    /// T054: Timestamp of last screen update completion
    last_screen_update: Option<Instant>,
}

/// Page views displayed on hardware controller
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageView {
    /// Show room volume, mute status, current stream
    Status,

    /// Show list of available streams for selection
    StreamSelection,

    /// Show connection status, server address
    Settings,
}

impl ApplicationState {
    /// Create a new ApplicationState with loaded configuration
    pub fn new(config: ConnectionSettings) -> Self {
        Self {
            config,
            room: None,
            streams: Vec::new(),
            hardware_connected: false,
            server_connected: false,
            selected_stream_index: 0,
            current_page: PageView::Status,
            last_state_change: None,
            last_screen_update: None,
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
        self.streams = streams;
    }

    /// T049: Handle ClientVolumeChanged event
    /// Returns true if state changed and screen refresh is needed
    pub fn handle_volume_changed(&mut self, client_id: &str, volume: u8, muted: bool) -> bool {
        // Only update if this is our room's client
        if let Some(room) = &mut self.room {
            if room.client_id == client_id {
                let changed = room.volume != volume || room.muted != muted;
                if changed {
                    room.volume = volume;
                    room.muted = muted;
                    // T054: Mark state change timestamp
                    self.last_state_change = Some(Instant::now());
                    return true;
                }
            }
        }
        false
    }

    /// T050: Handle StreamChanged event
    /// Returns true if state changed and screen refresh is needed
    pub fn handle_stream_changed(&mut self, client_id: &str, stream_id: String) -> bool {
        // Only update if this is our room's client
        if let Some(room) = &mut self.room {
            if room.client_id == client_id {
                let changed = room.stream_id.as_ref() != Some(&stream_id);
                if changed {
                    room.stream_id = Some(stream_id);
                    // T054: Mark state change timestamp
                    self.last_state_change = Some(Instant::now());
                    return true;
                }
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
                if let Some(room) = &self.room {
                    if room.stream_id.as_ref() == Some(&stream_id.to_string()) {
                        // T054: Mark state change timestamp
                        self.last_state_change = Some(Instant::now());
                        return true;
                    }
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
            (self.last_state_change, self.last_screen_update) {

            // Screen update should happen after state change
            if screen_update >= state_change {
                let elapsed = screen_update.duration_since(state_change);
                let elapsed_ms = elapsed.as_millis();

                // Validate < 2 seconds (2000ms)
                let is_valid = elapsed_ms < 2000;

                if !is_valid {
                    eprintln!("WARNING: Screen update latency exceeded 2 seconds: {}ms", elapsed_ms);
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

    /// T065: Handle knob rotation events
    /// Returns Some(new_volume) if volume should be changed, None otherwise
    pub fn handle_knob_rotated(&self, knob_id: u8, delta: i8) -> Option<(String, u8)> {
        // Only knob 0 controls volume
        if knob_id != 0 {
            return None;
        }

        // Get current room state
        let room = self.room.as_ref()?;

        // T058 & T059: Calculate new volume (delta * 5%), clamped to 0-100
        let volume_change = delta as i32 * 5;
        let new_volume = (room.volume as i32 + volume_change).clamp(0, 100) as u8;

        // Only return if volume actually changed
        if new_volume != room.volume {
            Some((room.client_id.clone(), new_volume))
        } else {
            None
        }
    }

    /// T067: Handle button press events for mute toggle
    /// Returns Some(new_muted_state) if button 0 was pressed, None otherwise
    pub fn handle_button_pressed(&self, button_id: u8) -> Option<(String, bool)> {
        // Only button 0 toggles mute
        if button_id != 0 {
            return None;
        }

        // Get current room state
        let room = self.room.as_ref()?;

        // Toggle muted state
        let new_muted = !room.muted;
        Some((room.client_id.clone(), new_muted))
    }
}
