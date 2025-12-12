// Snapcast types - Data structures for room state and audio streams

/// Represents the current state of a Snapcast room/client
#[derive(Debug, Clone)]
pub struct RoomState {
    /// Snapcast client ID
    pub client_id: String,

    /// Human-readable room name
    pub name: String,

    /// Volume level (0-100)
    pub volume: u8,

    /// Whether audio is muted
    pub muted: bool,

    /// Whether client is connected to server
    pub connected: bool,

    /// Currently assigned audio stream ID
    pub stream_id: Option<String>,

    /// Audio latency in milliseconds
    pub latency: Option<u32>,
}

/// Represents an audio source available on the Snapcast server
#[derive(Debug, Clone)]
pub struct AudioStream {
    /// Unique stream identifier
    pub stream_id: String,

    /// Display name
    pub name: String,

    /// Current stream status
    pub status: StreamStatus,

    /// Current track metadata (if available)
    pub metadata: Option<StreamMetadata>,
}

/// Stream playback status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamStatus {
    /// Stream is actively playing audio
    Playing,

    /// Stream exists but no audio
    Idle,

    /// Status cannot be determined
    Unknown,
}

/// Metadata for currently playing audio
#[derive(Debug, Clone)]
pub struct StreamMetadata {
    /// Artist name
    pub artist: Option<String>,

    /// Track title
    pub title: Option<String>,

    /// Album name
    pub album: Option<String>,
}

/// Events received from Snapcast server
#[derive(Debug, Clone)]
pub enum SnapcastEvent {
    /// Volume or mute status changed for a client
    ClientVolumeChanged {
        client_id: String,
        volume: u8,
        muted: bool,
    },

    /// Client came online
    ClientConnected { client_id: String },

    /// Client went offline
    ClientDisconnected { client_id: String },

    /// Client assigned to different stream
    StreamChanged {
        client_id: String,
        stream_id: String,
    },

    /// Lost connection to Snapcast server
    ServerDisconnected,

    /// Connection to Snapcast server restored
    ServerReconnected,
}
