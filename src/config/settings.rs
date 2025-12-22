use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// Persistent configuration for server connectivity and room assignment
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ConnectionSettings {
    pub server: ServerConfig,
    pub room: RoomConfig,
    pub homeassistant: Option<HomeAssistantConfig>,
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

        // Validate Home Assistant config if present
        if let Some(ref ha_config) = self.homeassistant {
            ha_config.validate()?;
        }

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

/// Configuration for Home Assistant MQTT integration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HomeAssistantConfig {
    /// MQTT broker address (hostname or IP)
    pub broker_address: String,

    /// MQTT broker port (typically 1883)
    pub broker_port: u16,

    /// Optional MQTT authentication username
    pub username: Option<String>,

    /// Optional MQTT authentication password
    pub password: Option<String>,

    /// MQTT topic for IR Blaster commands (e.g., "tasmota_17DD9F/cmnd/irsend")
    pub ir_blaster_topic: String,

    /// Amplifier power entity configuration
    pub amplifier: AmplifierPowerEntityConfig,
}

impl HomeAssistantConfig {
    /// Validate Home Assistant configuration
    pub fn validate(&self) -> Result<()> {
        anyhow::ensure!(
            !self.broker_address.is_empty(),
            "MQTT broker address cannot be empty"
        );

        anyhow::ensure!(
            self.broker_port > 0,
            "MQTT broker port must be greater than 0"
        );

        anyhow::ensure!(
            !self.ir_blaster_topic.is_empty(),
            "IR Blaster MQTT topic cannot be empty"
        );

        self.amplifier.validate()?;

        Ok(())
    }
}

/// Configuration for amplifier power entity in Home Assistant
/// Note: Source selection is managed locally, not via Home Assistant entity
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AmplifierPowerEntityConfig {
    /// Power switch entity ID (e.g., "switch.amplifier_power")
    pub power_entity: String,
}

impl AmplifierPowerEntityConfig {
    /// Validate entity ID format
    pub fn validate(&self) -> Result<()> {
        // Validate entity ID format: {domain}.{object_id}
        anyhow::ensure!(
            self.power_entity.contains('.'),
            "Power entity ID must be in format 'domain.object_id'"
        );

        // Validate power entity domain
        let power_domain = self.power_entity.split('.').next().unwrap_or("");
        anyhow::ensure!(
            power_domain == "switch",
            "Power entity domain must be 'switch'"
        );

        Ok(())
    }
}
