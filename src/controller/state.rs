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
