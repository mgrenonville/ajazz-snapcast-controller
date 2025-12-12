// Hardware device - USB HID device detection and management

use crate::hardware::{HardwareError, events::HardwareEvent};
use ajazz_sdk::{AsyncAjazz, list_devices, new_hidapi};
use std::{sync::Arc, time::Duration};
use tokio::sync::mpsc;

/// USB HID device manager for Ajazz controller
pub struct DeviceManager {
    /// Current device connection (None if disconnected)
    device: Option<Arc<AsyncAjazz>>,

    /// Channel to send hardware events
    event_tx: mpsc::UnboundedSender<HardwareEvent>,
}

impl DeviceManager {
    /// Create a new device manager
    pub fn new(event_tx: mpsc::UnboundedSender<HardwareEvent>) -> Self {
        Self {
            device: None,
            event_tx,
        }
    }

    /// Detect and connect to USB HID device
    pub async fn detect_device(&mut self) -> Result<(), HardwareError> {
        // List all connected Ajazz devices
        let hid = new_hidapi()
            .map_err(|e| HardwareError::SdkError(format!("Failed to list devices: {}", e)))?;

        let devices = list_devices(&hid);

        if devices.is_empty() {
            return Err(HardwareError::DeviceNotFound);
        }
        // Connect to the first available device

        let (kind, serial) = devices.first().unwrap();

        let device = AsyncAjazz::connect_with_retries(&hid, *kind, serial, 10)
            .map_err(|e| HardwareError::SdkError(format!("Failed to connect: {}", e)))?;

        println!(
            "Connected to '{}' with firmware version '{}'",
            device.serial_number().await?,
            device.firmware_version().await?
        );
        let _ = device.clear_all_button_images().await;
        let _ = device.set_brightness(100).await;

        self.device = Some(Arc::new(device));

        // Emit DeviceConnected event
        let _ = self.event_tx.send(HardwareEvent::DeviceConnected);

        Ok(())
    }

    /// Check if device is currently connected
    pub fn is_connected(&self) -> bool {
        self.device.is_some()
    }

    /// Get reference to the connected device
    pub fn device(&self) -> Option<&Arc<AsyncAjazz>> {
        self.device.as_ref()
    }

    /// Get mutable reference to the connected device
    pub fn device_mut(&mut self) -> Option<&mut Arc<AsyncAjazz>> {
        self.device.as_mut()
    }

    /// Handle device disconnection
    pub fn handle_disconnection(&mut self) {
        println!("Disconnection detected...");
        if self.device.is_some() {
            self.device = None;
            let _ = self.event_tx.send(HardwareEvent::DeviceDisconnected);
        }
    }

    /// Poll for device connection with retry logic
    pub async fn wait_for_device(&mut self, retry_interval: Duration) {
        loop {
            match self.detect_device().await {
                Ok(_) => {
                    break;
                }
                Err(HardwareError::DeviceNotFound) => {
                    // Device not found, wait and retry
                    tokio::time::sleep(retry_interval).await;
                }
                Err(e) => {
                    // Other error, log and retry
                    eprintln!("Error detecting device: {}", e);
                    tokio::time::sleep(retry_interval).await;
                }
            }
        }
    }
}

/// Continuously monitor for device connection/disconnection
pub async fn device_monitor_loop(
    mut manager: DeviceManager,
    poll_interval: Duration,
) -> Result<(), HardwareError> {
    // Initial device detection
    manager.wait_for_device(poll_interval).await;

    // Monitor loop
    loop {
        tokio::time::sleep(poll_interval).await;

        if manager.is_connected() {
            // Check if device is still connected
            if let Some(device) = manager.device() {
                match device
                    .keep_alive()
                    .await
                {
                    Ok(_) => {
                        // Device still connected
                        device.set_button_image_data(key, image)
                        continue;
                    }
                    Err(_) => {
                        // Device disconnected
                        manager.handle_disconnection();
                    }
                }
            }

            // Wait for reconnection
            manager.wait_for_device(poll_interval).await;
        }
    }
}
