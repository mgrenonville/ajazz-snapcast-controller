// Snapcast client - TCP connection and communication with Snapcast server

use crate::snapcast::{types::SnapcastEvent, SnapcastError};
use snapcast_control::{ClientError, SnapcastConnection};
use std::net::SocketAddr;
use tokio::sync::mpsc;

/// Snapcast server connection manager
pub struct SnapcastClient {
    /// Server address
    address: SocketAddr,

    /// Active connection (None if disconnected)
    connection: Option<SnapcastConnection>,

    /// Channel to send Snapcast events
    event_tx: mpsc::UnboundedSender<SnapcastEvent>,
}

impl SnapcastClient {
    /// Create a new Snapcast client
    pub fn new(
        address: SocketAddr,
        event_tx: mpsc::UnboundedSender<SnapcastEvent>,
    ) -> Self {
        Self {
            address,
            connection: None,
            event_tx,
        }
    }

    /// Connect to Snapcast server
    pub async fn connect(&mut self) -> Result<(), SnapcastError> {
        let connection = SnapcastConnection::open(self.address)
            .await;

        self.connection = Some(connection);

        // Emit ServerReconnected event
        let _ = self.event_tx.send(SnapcastEvent::ServerReconnected);

        Ok(())
    }

    /// Check if client is currently connected
    pub fn is_connected(&self) -> bool {
        self.connection.is_some()
    }

    /// Get reference to the active connection
    pub fn connection(&self) -> Option<&SnapcastConnection> {
        self.connection.as_ref()
    }

    /// Get mutable reference to the active connection
    pub fn connection_mut(&mut self) -> Option<&mut SnapcastConnection> {
        self.connection.as_mut()
    }

    /// Handle server disconnection
    pub fn handle_disconnection(&mut self) {
        if self.connection.is_some() {
            self.connection = None;
            let _ = self.event_tx.send(SnapcastEvent::ServerDisconnected);
        }
    }

    /// Wait for successful connection with retry logic
    pub async fn wait_for_connection(&mut self, retry_interval: std::time::Duration) {
        loop {
            match self.connect().await {
                Ok(_) => {
                    break;
                }
                Err(e) => {
                    eprintln!("Failed to connect to Snapcast server: {}", e);
                    tokio::time::sleep(retry_interval).await;
                }
            }
        }
    }

    /// Convert snapcast_control ClientError to SnapcastError
    fn convert_error(error: ClientError) -> SnapcastError {
        match error {
            ClientError::Io(e) => SnapcastError::ConnectionFailed(e.to_string()),
            ClientError::Snapcast(e) => SnapcastError::RpcError(e.to_string()),
            ClientError::Deserialization(e) => SnapcastError::InvalidResponse(e.to_string()),
            ClientError::JsonDeserialization(e) => {
                SnapcastError::InvalidResponse(e.to_string())
            }
            ClientError::Unknown(e) => SnapcastError::ControlError(e),
        }
    }

    /// Request server status (Server.GetStatus)
    /// This triggers the server to send the full server state which will be parsed
    /// and stored in the connection's state object
    pub async fn get_server_status(&mut self) -> Result<(), SnapcastError> {
        let connection = self
            .connection
            .as_mut()
            .ok_or_else(|| SnapcastError::ConnectionFailed("Not connected".to_string()))?;

        connection
            .server_get_status()
            .await
            .map_err(Self::convert_error)?;

        Ok(())
    }

    /// Extract room state for a specific client from the server state
    /// Returns None if client is not found or not connected
    pub fn get_room_state(&self, client_id: &str) -> Option<crate::snapcast::types::RoomState> {
        let connection = self.connection.as_ref()?;

        // Get client info from state
        let client = connection.state.clients.get(client_id)?;

        // Find which group the client belongs to
        let group = connection.state.groups.iter()
            .find(|g| g.value().clients.contains(client_id))?;

        // Build RoomState from client and group info
        Some(crate::snapcast::types::RoomState {
            client_id: client.id.clone(),
            name: client.config.name.clone(),
            volume: client.config.volume.percent.min(100) as u8,
            muted: client.config.volume.muted,
            connected: client.connected,
            stream_id: Some(group.value().stream_id.clone()),
            latency: Some(client.config.latency as u32),
        })
    }

    /// Find the group ID that contains a specific client
    pub fn find_group_for_client(&self, client_id: &str) -> Option<String> {
        let connection = self.connection.as_ref()?;

        connection.state.groups.iter()
            .find(|g| g.value().clients.contains(client_id))
            .map(|g| g.key().clone())
    }

    /// Get all available streams from the server state
    pub fn get_streams(&self) -> Vec<crate::snapcast::types::AudioStream> {
        use snapcast_control::stream::StreamStatus as SnapcastStreamStatus;

        let connection = match self.connection.as_ref() {
            Some(c) => c,
            None => return Vec::new(),
        };

        connection.state.streams.iter()
            .filter_map(|entry| {
                let stream = entry.value().as_ref()?;

                // Convert snapcast_control StreamStatus to our StreamStatus
                let status = match stream.status {
                    SnapcastStreamStatus::Playing => crate::snapcast::types::StreamStatus::Playing,
                    SnapcastStreamStatus::Idle => crate::snapcast::types::StreamStatus::Idle,
                    _ => crate::snapcast::types::StreamStatus::Unknown,
                };

                // Extract stream name from URI
                // Try to get name from query params first, otherwise use path
                let name = stream.uri.query.get("name")
                    .cloned()
                    .unwrap_or_else(|| stream.uri.path.clone());

                Some(crate::snapcast::types::AudioStream {
                    stream_id: stream.id.clone(),
                    name,
                    status,
                    // Metadata fields are private in snapcast_control::stream::StreamMetadata
                    // We cannot access them directly, so we set to None for now
                    // TODO: Consider using serde_json to serialize/deserialize to access fields
                    metadata: None,
                })
            })
            .collect()
    }

    /// Set client volume
    pub async fn set_client_volume(
        &mut self,
        client_id: String,
        volume: u8,
        muted: bool,
    ) -> Result<(), SnapcastError> {
        let connection = self
            .connection
            .as_mut()
            .ok_or_else(|| SnapcastError::ConnectionFailed("Not connected".to_string()))?;

        let volume_params = snapcast_control::client::ClientVolume {
            percent: volume as usize,
            muted,
        };

        connection
            .client_set_volume(client_id, volume_params)
            .await
            .map_err(Self::convert_error)?;

        Ok(())
    }

    /// Set group stream
    pub async fn set_group_stream(
        &mut self,
        group_id: String,
        stream_id: String,
    ) -> Result<(), SnapcastError> {
        let connection = self
            .connection
            .as_mut()
            .ok_or_else(|| SnapcastError::ConnectionFailed("Not connected".to_string()))?;

        connection
            .group_set_stream(group_id, stream_id)
            .await
            .map_err(Self::convert_error)?;

        Ok(())
    }

    /// Receive and process messages from server
    pub async fn receive_message(&mut self) -> Result<Option<()>, SnapcastError> {
        let connection = self
            .connection
            .as_mut()
            .ok_or_else(|| SnapcastError::ConnectionFailed("Not connected".to_string()))?;

        match connection.recv().await {
            Some(Ok(_message)) => {
                // Message received and processed
                // The snapcast_control library handles state updates automatically
                Ok(Some(()))
            }
            Some(Err(e)) => {
                // Error receiving message
                Err(Self::convert_error(e))
            }
            None => {
                // Connection closed
                self.handle_disconnection();
                Ok(None)
            }
        }
    }
}

/// Message loop for receiving Snapcast server events
pub async fn message_loop(
    mut client: SnapcastClient,
    reconnect_interval: std::time::Duration,
) -> Result<(), SnapcastError> {
    // Initial connection
    client.wait_for_connection(reconnect_interval).await;

    // Request initial server status
    let _ = client.get_server_status().await;

    loop {
        if !client.is_connected() {
            // Wait for reconnection
            client.wait_for_connection(reconnect_interval).await;

            // Request server status after reconnection
            let _ = client.get_server_status().await;
        }

        // Receive and process messages
        match client.receive_message().await {
            Ok(Some(())) => {
                // Message processed successfully
                continue;
            }
            Ok(None) => {
                // Connection closed, reconnect
                continue;
            }
            Err(e) => {
                eprintln!("Error receiving message: {}", e);
                client.handle_disconnection();
            }
        }
    }
}
