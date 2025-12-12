use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// Persistent configuration for server connectivity and room assignment
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ConnectionSettings {
    pub server: ServerConfig,
    pub room: RoomConfig,
}

impl ConnectionSettings {
    /// Load configuration from TOML file
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read config file: {:?}", path.as_ref()))?;

        let settings: ConnectionSettings =
            toml::from_str(&content).context("Failed to parse TOML configuration")?;

        settings.validate()?;

        Ok(settings)
    }

    /// Validate configuration settings
    pub fn validate(&self) -> Result<()> {
        // Validate server address is not empty
        anyhow::ensure!(
            !self.server.address.is_empty(),
            "Server address cannot be empty"
        );

        // Validate port is in valid range (1-65535)
        anyhow::ensure!(self.server.port > 0, "Server port must be greater than 0");

        // Validate client_id is not empty
        anyhow::ensure!(
            !self.room.client_id.is_empty(),
            "Room client_id cannot be empty"
        );

        Ok(())
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServerConfig {
    /// IP address or hostname of Snapcast server
    pub address: String,

    /// TCP port for JSON-RPC API (default: 1705)
    pub port: u16,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RoomConfig {
    /// Snapcast client ID this controller manages
    pub client_id: String,
}
