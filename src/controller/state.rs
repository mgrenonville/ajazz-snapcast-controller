// Controller state - Central state machine for the application

use crate::config::settings::ConnectionSettings;
use crate::snapcast::types::{AudioStream, RoomState};

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
}
