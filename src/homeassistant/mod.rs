/// MQTT client for Home Assistant integration
pub mod client;

/// IR command builders
pub mod commands;

/// RC5 IR protocol codes
pub mod ir_codes;

/// Type definitions for Home Assistant entities and events
pub mod types;

// Re-export commonly used types
pub use types::{
    AmplifierSource, AmplifierState, EntityAvailability, HomeAssistantCommand,
    HomeAssistantConnection, HomeAssistantEvent, IrCommand,
};
