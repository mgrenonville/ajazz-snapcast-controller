// Hardware module - USB HID controller integration

use ajazz_sdk::AjazzError;
use thiserror::Error;

pub mod device;
pub mod display;
pub mod events;

/// Hardware-related errors
#[derive(Debug, Error)]
pub enum HardwareError {
    #[error("USB HID device not found")]
    DeviceNotFound,

    #[error("Failed to read from device: {0}")]
    ReadError(String),

    #[error("Failed to write to device: {0}")]
    WriteError(String),

    #[error("Device disconnected")]
    DeviceDisconnected,

    #[error("Invalid device response: {0}")]
    InvalidResponse(String),

    #[error("ajazz_sdk error: {0}")]
    SdkError(String),
}

impl From<AjazzError> for HardwareError {
    fn from(value: AjazzError) -> Self {
        Self::SdkError(value.to_string())
    }
}
