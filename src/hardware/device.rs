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

/// Connection handler for managing hardware device connection lifecycle
pub struct DeviceConnectionHandler {
    manager: DeviceManager,
    base_poll_interval: Duration,
    max_poll_interval: Duration,
    current_poll_interval: Duration,
}

impl DeviceConnectionHandler {
    /// Create a new device connection handler
    pub fn new(manager: DeviceManager, base_poll_interval: Duration) -> Self {
        Self {
            manager,
            base_poll_interval,
            max_poll_interval: Duration::from_secs(10),
            current_poll_interval: base_poll_interval,
        }
    }

    /// Attempt to detect and connect to device
    async fn try_connect(&mut self) -> Result<(), HardwareError> {
        match self.manager.detect_device().await {
            Ok(_) => {
                // Reset poll interval on successful connection
                self.current_poll_interval = self.base_poll_interval;
                Ok(())
            }
            Err(e) => {
                // Exponential backoff on failure
                self.current_poll_interval = std::cmp::min(
                    self.current_poll_interval * 2,
                    self.max_poll_interval,
                );
                Err(e)
            }
        }
    }

    /// Wait for device connection with exponential backoff
    async fn ensure_connected(&mut self) {
        // T028: Display waiting message on console (can't show on device before it's connected)
        println!("Waiting for hardware...");

        while !self.manager.is_connected() {
            match self.try_connect().await {
                Ok(_) => {
                    println!("Hardware device connected successfully");
                    break;
                }
                Err(HardwareError::DeviceNotFound) => {
                    // Device not found is expected, just wait
                    tokio::time::sleep(self.current_poll_interval).await;
                }
                Err(e) => {
                    eprintln!(
                        "Device connection failed: {}, retrying in {:?}",
                        e, self.current_poll_interval
                    );
                    tokio::time::sleep(self.current_poll_interval).await;
                }
            }
        }
    }

    /// Check if device is still alive
    async fn check_device_alive(&self) -> bool {
        if let Some(device) = self.manager.device() {
            match device.keep_alive().await {
                Ok(_) => true,
                Err(_) => false,
            }
        } else {
            false
        }
    }

    /// Handle connection/disconnection events and reconnection
    pub async fn handle_connection_lifecycle(&mut self) -> Result<(), HardwareError> {
        // Initial connection
        self.ensure_connected().await;

        loop {
            // Ensure we're connected before checking device health
            if !self.manager.is_connected() {
                self.ensure_connected().await;
            }

            // Wait before next health check
            tokio::time::sleep(self.base_poll_interval).await;

            // Check if device is still alive
            if self.manager.is_connected() && !self.check_device_alive().await {
                eprintln!("Hardware device health check failed");
                self.manager.handle_disconnection();
                // Will reconnect on next iteration
            }
        }
    }

    /// Get reference to the manager
    pub fn manager(&self) -> &DeviceManager {
        &self.manager
    }

    /// Get mutable reference to the manager
    pub fn manager_mut(&mut self) -> &mut DeviceManager {
        &mut self.manager
    }
}

/// Continuously monitor for device connection/disconnection
/// This is a convenience function that creates a DeviceConnectionHandler and runs it
pub async fn device_monitor_loop(
    manager: DeviceManager,
    poll_interval: Duration,
) -> Result<(), HardwareError> {
    let mut handler = DeviceConnectionHandler::new(manager, poll_interval);
    handler.handle_connection_lifecycle().await
}
