use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

/// Amplifier input source selection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum AmplifierSource {
    /// Phono input (turntable)
    #[default]
    Phono,
    /// CD player input
    CD,
    /// Spotify/streaming input
    Spotify,
    /// Source 4 input
    Source4,
    /// Source 5 input
    Source5,
}

impl AmplifierSource {
    /// Get the IR code for this source
    pub fn ir_code(&self) -> u16 {
        use crate::homeassistant::ir_codes::sources;
        match self {
            AmplifierSource::Phono => sources::PHONO,
            AmplifierSource::CD => sources::CD,
            AmplifierSource::Spotify => sources::SPOTIFY,
            AmplifierSource::Source4 => sources::SOURCE_4,
            AmplifierSource::Source5 => sources::SOURCE_5,
        }
    }

    /// Get display name for this source
    pub fn display_name(&self) -> &'static str {
        match self {
            AmplifierSource::Phono => "Phono",
            AmplifierSource::CD => "CD",
            AmplifierSource::Spotify => "Spotify",
            AmplifierSource::Source4 => "Source 4",
            AmplifierSource::Source5 => "Source 5",
        }
    }

    /// Get all available sources
    pub fn all() -> [AmplifierSource; 5] {
        [
            AmplifierSource::Phono,
            AmplifierSource::CD,
            AmplifierSource::Spotify,
            AmplifierSource::Source4,
            AmplifierSource::Source5,
        ]
    }
}

/// RC5 IR command structure for Tasmota IR Blaster
#[derive(Debug, Clone, Serialize)]
pub struct IrCommand {
    /// Protocol name (always "RC5" for this amplifier)
    #[serde(rename = "Protocol")]
    pub protocol: String,

    /// Number of bits (always 12 for RC5)
    #[serde(rename = "Bits")]
    pub bits: u8,

    /// Data code (hex value for the command)
    #[serde(rename = "Data")]
    pub data: String,

    /// Repeat count (0 = send once)
    #[serde(rename = "Repeat")]
    pub repeat: u8,
}

impl IrCommand {
    /// Create a new IR command with the given data code
    pub fn new(data_code: u16) -> Self {
        use crate::homeassistant::ir_codes::protocol;
        Self {
            protocol: protocol::NAME.to_string(),
            bits: protocol::BITS,
            data: format!("0x{:X}", data_code),
            repeat: protocol::REPEAT,
        }
    }

    /// Convert to JSON string for MQTT payload
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
}

/// Zigbee2MQTT state payload structure
/// Only deserializes the "state" field, ignoring all other fields
#[derive(Debug, Deserialize)]
pub struct Zigbee2MqttStatePayload {
    /// Device state (e.g., "ON", "OFF")
    pub state: String,
}

/// Entity availability status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityAvailability {
    /// Entity is reachable and responding
    Available,
    /// Entity explicitly marked unavailable by Home Assistant
    Unavailable,
    /// Availability status unknown (no recent updates)
    Unknown,
}

/// Represents the current state of the amplifier
#[derive(Debug, Clone)]
pub struct AmplifierState {
    /// Amplifier power state (None = unknown, Some(true) = on, Some(false) = off)
    pub power_on: Option<bool>,

    /// Currently selected input source (tracked locally, not synced from Home Assistant)
    pub selected_source: AmplifierSource,

    /// Estimated volume level (0-100, tracked locally via IR commands)
    /// Since IR is one-way, this is an estimate based on sent commands
    pub volume: u8,

    /// Timestamp of last state update
    pub last_updated: Instant,

    /// Whether the power entity is reachable in Home Assistant
    pub availability: EntityAvailability,
}

impl AmplifierState {
    /// Create a new AmplifierState with unknown power state and default source
    pub fn new() -> Self {
        Self {
            power_on: None,
            selected_source: AmplifierSource::default(),
            volume: 50, // Start at middle volume
            last_updated: Instant::now(),
            availability: EntityAvailability::Unknown,
        }
    }

    /// Update power state
    pub fn set_power(&mut self, on: bool) {
        self.power_on = Some(on);
        self.last_updated = Instant::now();
        self.availability = EntityAvailability::Available;
    }

    /// Update selected source (local state only)
    pub fn set_source(&mut self, source: AmplifierSource) {
        self.selected_source = source;
        self.last_updated = Instant::now();
    }

    /// Increase volume by 1 (capped at 100)
    pub fn increase_volume(&mut self) {
        self.volume = self.volume.saturating_add(1).min(100);
        self.last_updated = Instant::now();
    }

    /// Decrease volume by 1 (capped at 0)
    pub fn decrease_volume(&mut self) {
        self.volume = self.volume.saturating_sub(1);
        self.last_updated = Instant::now();
    }

    /// Set volume directly (capped at 0-100)
    pub fn set_volume(&mut self, volume: u8) {
        self.volume = volume.min(100);
        self.last_updated = Instant::now();
    }

    /// Update availability status
    pub fn set_availability(&mut self, availability: EntityAvailability) {
        self.availability = availability;
        self.last_updated = Instant::now();
    }

    /// Check if state is stale (no update for >30 seconds)
    pub fn is_stale(&self) -> bool {
        self.last_updated.elapsed() > Duration::from_secs(30)
    }
}

impl Default for AmplifierState {
    fn default() -> Self {
        Self::new()
    }
}

/// Represents the MQTT broker connection state
#[derive(Debug, Clone)]
pub struct HomeAssistantConnection {
    /// MQTT broker hostname or IP
    pub broker_address: String,

    /// MQTT broker port
    pub broker_port: u16,

    /// MQTT authentication username
    pub username: Option<String>,

    /// MQTT client identifier
    pub client_id: String,

    /// Current connection status
    pub connected: bool,

    /// Timestamp of last connection attempt
    pub last_connect_attempt: Option<Instant>,

    /// Current reconnection backoff delay
    pub reconnect_delay: Duration,
}

impl HomeAssistantConnection {
    /// Create a new connection configuration
    pub fn new(broker_address: String, broker_port: u16, username: Option<String>) -> Self {
        // Generate unique client ID with random suffix
        let client_id = format!("snapcast-controller-{}", rand_suffix());

        Self {
            broker_address,
            broker_port,
            username,
            client_id,
            connected: false,
            last_connect_attempt: None,
            reconnect_delay: Duration::from_secs(1),
        }
    }

    /// Mark connection as established
    pub fn set_connected(&mut self) {
        self.connected = true;
        self.reconnect_delay = Duration::from_secs(1); // Reset backoff
    }

    /// Mark connection as lost
    pub fn set_disconnected(&mut self) {
        self.connected = false;
        self.last_connect_attempt = Some(Instant::now());
    }

    /// Increase reconnection backoff delay (exponential backoff, max 60s)
    pub fn increase_backoff(&mut self) {
        self.reconnect_delay = (self.reconnect_delay * 2).min(Duration::from_secs(60));
    }
}

/// Generate a random 6-character suffix for client ID
fn rand_suffix() -> String {
    use std::time::SystemTime;
    let timestamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_millis();
    format!("{:06x}", timestamp % 0xFFFFFF)
}

/// Events emitted by the Home Assistant client
#[derive(Debug, Clone)]
pub enum HomeAssistantEvent {
    /// MQTT broker connection established
    BrokerConnected,

    /// MQTT broker connection lost
    BrokerDisconnected,

    /// Entity state changed
    EntityStateChanged {
        /// Full entity ID (e.g., "switch.amplifier_power")
        entity_id: String,
        /// New state value (e.g., "on", "off", source name)
        state: String,
    },

    /// Entity availability changed
    EntityAvailabilityChanged {
        /// Full entity ID
        entity_id: String,
        /// Whether entity is available
        available: bool,
    },

    /// Command successfully published to broker
    CommandAcknowledged {
        /// Entity ID the command was sent to
        entity_id: String,
    },

    /// Command failed to publish or timed out
    CommandFailed {
        /// Entity ID the command was for
        entity_id: String,
        /// Error message
        error: String,
    },
}

/// Commands sent to the Home Assistant client
#[derive(Debug, Clone)]
pub enum HomeAssistantCommand {
    /// Toggle amplifier power (on→off or off→on)
    TogglePower,

    /// Set amplifier power to specific state
    SetPower { on: bool },

    /// Change amplifier input source (sends IR command)
    SelectSource { source: AmplifierSource },

    /// Increase volume (sends IR command)
    VolumeUp,

    /// Decrease volume (sends IR command)
    VolumeDown,

    /// Gracefully disconnect from MQTT broker
    Disconnect,
}
