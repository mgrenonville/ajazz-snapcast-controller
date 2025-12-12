// Controller commands - Commands sent from controller to Snapcast server

/// Commands sent from controller to Snapcast server
#[derive(Debug, Clone)]
pub enum ControllerCommand {
    /// Adjust volume for a client (volume: 0-100)
    SetVolume { client_id: String, volume: u8 },

    /// Mute or unmute a client
    SetMute { client_id: String, muted: bool },

    /// Assign a client to a different stream
    AssignStream {
        client_id: String,
        stream_id: String,
    },

    /// Request full server status
    GetStatus,
}
