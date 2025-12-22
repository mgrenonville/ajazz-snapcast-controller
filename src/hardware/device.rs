// Hardware device - USB HID device detection and management

use crate::hardware::{
    HardwareError,
    display::DisplayManager,
    events::{HardwareCommand, HardwareEvent},
};
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
    manager: std::sync::Arc<tokio::sync::Mutex<DeviceManager>>,
    base_poll_interval: Duration,
    max_poll_interval: Duration,
    current_poll_interval: Duration,
    /// Event reader for polling hardware events
    event_reader: Option<std::sync::Arc<ajazz_sdk::asynchronous::AsyncDeviceStateReader>>,
}

impl DeviceConnectionHandler {
    /// Create a new device connection handler
    pub fn new(
        manager: std::sync::Arc<tokio::sync::Mutex<DeviceManager>>,
        base_poll_interval: Duration,
    ) -> Self {
        Self {
            manager,
            base_poll_interval,
            max_poll_interval: Duration::from_secs(10),
            current_poll_interval: base_poll_interval,
            event_reader: None,
        }
    }

    /// Attempt to detect and connect to device
    async fn try_connect(&mut self) -> Result<(), HardwareError> {
        let mut manager = self.manager.lock().await;
        match manager.detect_device().await {
            Ok(_) => {
                // Reset poll interval on successful connection
                self.current_poll_interval = self.base_poll_interval;

                // Create event reader for the connected device
                if let Some(device) = manager.device() {
                    self.event_reader = Some(device.get_reader());
                }

                Ok(())
            }
            Err(e) => {
                // Exponential backoff on failure
                self.current_poll_interval =
                    std::cmp::min(self.current_poll_interval * 2, self.max_poll_interval);
                Err(e)
            }
        }
    }

    /// Wait for device connection with exponential backoff
    async fn ensure_connected(&mut self) {
        // T028: Display waiting message on console (can't show on device before it's connected)
        println!("Waiting for hardware...");

        loop {
            let is_connected = {
                let manager = self.manager.lock().await;
                manager.is_connected()
            };

            if is_connected {
                break;
            }

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
        let manager = self.manager.lock().await;
        if let Some(device) = manager.device() {
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

        let mut health_check_interval = tokio::time::interval(self.base_poll_interval);
        let mut event_poll_interval = tokio::time::interval(Duration::from_millis(10));

        loop {
            eprintln!("handle_connection_lifecycle loop");
            tokio::select! {
                // Health check timer
                _ = health_check_interval.tick() => {
                    // Ensure we're connected before checking device health
                    let is_connected = {
                        let manager = self.manager.lock().await;
                        manager.is_connected()
                    };

                    if !is_connected {
                        self.ensure_connected().await;
                        continue;
                    }

                    // Check if device is still alive
                    if !self.check_device_alive().await {
                        eprintln!("Hardware device health check failed");
                        let mut manager = self.manager.lock().await;
                        manager.handle_disconnection();
                        // Will reconnect on next iteration
                    }
                }

                // Poll for hardware events (T055-T057)
                _ = event_poll_interval.tick() => {
                    if let Err(e) = self.poll_hardware_events().await {
                        eprintln!("Error polling hardware events: {}", e);
                    }
                }
            }
        }
    }

    /// T055-T057: Poll for hardware events (knobs, buttons, encoders)
    async fn poll_hardware_events(&mut self) -> Result<(), HardwareError> {
        use ajazz_sdk::Event;

        // Check if we have an event reader
        if let Some(reader) = &self.event_reader {
            // Read events with a high poll rate for responsiveness
            match reader.read(100.0).await {
                Ok(events) => {
                    let manager = self.manager.lock().await;
                    for event in events {
                        match event {
                            // T055: Encoder/Knob twist events
                            Event::EncoderTwist(encoder_id, delta) => {
                                let _ = manager.event_tx.send(HardwareEvent::KnobRotated {
                                    knob_id: encoder_id,
                                    delta,
                                });
                            }

                            // T056: Page buttons
                            Event::ButtonDown(button_id) if button_id >= 6 => {
                                let _ = manager.event_tx.send(HardwareEvent::PageButtonPressed {
                                    page_id: button_id - 6,
                                });
                            }
                            // T056: Button press/release events
                            Event::ButtonDown(button_id) => {
                                let _ = manager
                                    .event_tx
                                    .send(HardwareEvent::ButtonPressed { button_id });
                            }

                            Event::ButtonUp(button_id) => {
                                let _ = manager
                                    .event_tx
                                    .send(HardwareEvent::ButtonReleased { button_id });
                            }

                            // T057: Encoder press/release (treat as page buttons)
                            Event::EncoderDown(_) => {}

                            // Ignore encoder up events for now
                            Event::EncoderUp(_) => {}
                        }
                    }
                    Ok(())
                }
                Err(e) => Err(HardwareError::ReadError(e.to_string())),
            }
        } else {
            // No reader available yet (device not connected)
            Ok(())
        }
    }
}

/// Handle display update commands in a separate task
pub async fn device_display_loop(
    manager: std::sync::Arc<tokio::sync::Mutex<DeviceManager>>,
    mut command_rx: tokio::sync::watch::Receiver<Option<HardwareCommand>>,
) -> Result<(), HardwareError> {
    let display_manager = DisplayManager::new()?;

    loop {
        eprintln!("device display loop");

        // Wait for display command
        if let Ok(()) = command_rx.changed().await {
            // Get the latest command (may have been updated multiple times)
            // Clone to drop the borrow guard before awaiting
            let command = command_rx.borrow_and_update().clone();
            eprintln!("received display loop: {:?}", command);

            if let Some(command) = command {
                // Lock manager to access device
                let manager_guard = manager.lock().await;
                if let Some(device) = manager_guard.device() {
                    match command {
                        HardwareCommand::UpdateStatusPage(layout) => {
                            eprintln!("Update status page: {:?}", layout);
                            if let Err(e) =
                                display_manager.render_status_page(device, &layout).await
                            {
                                eprintln!("Failed to update status page: {}", e);
                            }
                        }
                        HardwareCommand::UpdateStreamSelectionPage(layout) => {
                            eprintln!("Update stream selection page: {:?}", layout);
                            if let Err(e) = display_manager
                                .render_stream_selection_page(device, &layout)
                                .await
                            {
                                eprintln!("Failed to update stream selection page: {}", e);
                            }
                        }
                        HardwareCommand::ShowError(error_msg) => {
                            eprintln!("Showing error: {}", error_msg);
                            if let Err(e) = display_manager.render_error(device, &error_msg).await {
                                eprintln!("Failed to show error: {}", e);
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Continuously monitor for device connection/disconnection
pub async fn device_monitor_loop(
    manager: std::sync::Arc<tokio::sync::Mutex<DeviceManager>>,
    poll_interval: Duration,
) -> Result<(), HardwareError> {
    let mut handler = DeviceConnectionHandler::new(manager, poll_interval);

    handler.handle_connection_lifecycle().await
}
