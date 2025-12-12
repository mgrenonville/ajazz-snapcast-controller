// Snapcast module - Snapcast server integration

use thiserror::Error;

pub mod client;
pub mod types;
pub mod commands;

/// Snapcast-related errors
#[derive(Debug, Error)]
pub enum SnapcastError {
    #[error("Failed to connect to server: {0}")]
    ConnectionFailed(String),

    #[error("Server disconnected")]
    ServerDisconnected,

    #[error("Invalid room/client ID: {0}")]
    InvalidRoom(String),

    #[error("Command failed: {0}")]
    CommandError(String),

    #[error("JSON-RPC error: {0}")]
    RpcError(String),

    #[error("Invalid server response: {0}")]
    InvalidResponse(String),

    #[error("Room not found on server: {0}")]
    RoomNotFound(String),

    #[error("snapcast_control error: {0}")]
    ControlError(String),
}
