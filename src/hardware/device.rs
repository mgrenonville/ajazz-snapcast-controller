// Hardware device - USB HID device detection and management

use crate::hardware::{
    HardwareError,
    display::DisplayManager,
    events::{HardwareCommand, HardwareEvent},
};
use ajazz_sdk::{AsyncAjazz, list_devices, new_hidapi};
use std::{sync::Arc, time::Duration};
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

/// USB HID device manager for Ajazz controller
pub struct DeviceManager {
    /// Current device connection (None if disconnected)
    device: Option<Arc<AsyncAjazz>>,

    /// Channel to send hardware events
    event_tx: mpsc::UnboundedSender<HardwareEvent>,

    /// Sleep state tracking
    is_sleeping: bool,

    /// Last activity timestamp (for inactivity timeout)
    last_activity: std::time::Instant,

    /// Sleep timeout duration
    sleep_timeout: Duration,
}

impl DeviceManager {
    /// Create a new device manager
    pub fn new(event_tx: mpsc::UnboundedSender<HardwareEvent>, sleep_timeout: Duration) -> Self {
        info!("DeviceManager created with sleep timeout: {:?}", sleep_timeout);
        Self {
            device: None,
            event_tx,
            is_sleeping: false,
            last_activity: std::time::Instant::now(),
            sleep_timeout,
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

        info!(
            "Hardware device connected: serial='{}' firmware='{}'",
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
        info!("Hardware device disconnection detected");
        if self.device.is_some() {
            self.device = None;
            self.is_sleeping = false;
            let _ = self.event_tx.send(HardwareEvent::DeviceDisconnected);
        }
    }

    /// Reset inactivity timer (called on user activity)
    pub fn reset_activity_timer(&mut self) {
        debug!("Activity timer reset - device will sleep after {:?} of inactivity", self.sleep_timeout);
        self.last_activity = std::time::Instant::now();
    }

    /// Check if device should sleep due to inactivity
    pub fn should_sleep(&self) -> bool {
        let elapsed = self.last_activity.elapsed();
        let should_sleep = !self.is_sleeping && elapsed >= self.sleep_timeout;

        if should_sleep {
            debug!("Device should sleep: elapsed={:?}, timeout={:?}, is_sleeping={}",
                   elapsed, self.sleep_timeout, self.is_sleeping);
        }

        should_sleep
    }

    /// Put device to sleep
    pub async fn sleep(&mut self) -> Result<(), HardwareError> {
        if self.is_sleeping {
            debug!("Device already sleeping, skipping");
            return Ok(());
        }

        if let Some(device) = &self.device {
            info!("🌙 Putting device to sleep after {:?} of inactivity", self.last_activity.elapsed());
            device
                .sleep()
                .await
                .map_err(|e| HardwareError::WriteError(e.to_string()))?;
            self.is_sleeping = true;
            info!("✓ Device is now sleeping");
        } else {
            debug!("Cannot sleep - no device connected");
        }
        Ok(())
    }

    /// Wake device from sleep
    pub fn wake(&mut self) {
        if self.is_sleeping {
            info!("☀️ Device waking from sleep");
            self.is_sleeping = false;
            self.reset_activity_timer();
            info!("✓ Device is now awake");
        } else {
            debug!("Wake called but device was not sleeping");
        }
    }

    /// Check if device is sleeping
    pub fn is_sleeping(&self) -> bool {
        self.is_sleeping
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
                    error!("Error detecting hardware device: {}", e);
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
        info!("Waiting for hardware device...");

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
                    info!("Hardware device connected successfully");
                    break;
                }
                Err(HardwareError::DeviceNotFound) => {
                    // Device not found is expected, just wait
                    tokio::time::sleep(self.current_poll_interval).await;
                }
                Err(e) => {
                    warn!(
                        "Hardware device connection failed: {}, retrying in {:?}",
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
            (device.keep_alive().await).is_ok()
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
                        warn!("Hardware device health check failed");
                        let mut manager = self.manager.lock().await;
                        manager.handle_disconnection();
                        // Will reconnect on next iteration
                    }
                }

                // Poll for hardware events (T055-T057)
                _ = event_poll_interval.tick() => {
                    if let Err(e) = self.poll_hardware_events().await {
                        error!("Error polling hardware events: {}", e);
                    }
                }
            }
        }
    }

    /// Run sleep check loop in parallel (separate task)
    async fn sleep_check_loop(manager: std::sync::Arc<tokio::sync::Mutex<DeviceManager>>) {
        let mut sleep_check_interval = tokio::time::interval(Duration::from_secs(1));

        loop {
            sleep_check_interval.tick().await;

            let mut manager_guard = manager.lock().await;
            let elapsed = manager_guard.last_activity.elapsed();
            let timeout = manager_guard.sleep_timeout;
            let is_sleeping = manager_guard.is_sleeping();

            debug!("Sleep check: elapsed={:?}, timeout={:?}, is_sleeping={}, should_sleep={}",
                   elapsed, timeout, is_sleeping, manager_guard.should_sleep());

            if manager_guard.should_sleep() {
                debug!("Sleep conditions met, calling sleep()");
                if let Err(e) = manager_guard.sleep().await {
                    error!("Failed to put device to sleep: {}", e);
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
                    let mut manager = self.manager.lock().await;

                    if !events.is_empty() {
                        debug!("Received {} hardware events", events.len());
                    }

                    for event in events {
                        // Check if device is sleeping and this is the wake-up event
                        let was_sleeping = manager.is_sleeping();

                        match event {
                            // T055: Encoder/Knob twist events
                            Event::EncoderTwist(encoder_id, delta) => {
                                debug!("Event: EncoderTwist(id={}, delta={}), sleeping={}", encoder_id, delta, was_sleeping);
                                if was_sleeping {
                                    // Wake up and drop this event
                                    info!("🔔 Device waking from sleep - dropping knob rotation event (id={}, delta={})", encoder_id, delta);
                                    manager.wake();
                                } else {
                                    // Normal operation - send event and reset activity timer
                                    debug!("Processing knob rotation in normal mode - resetting activity timer");
                                    manager.reset_activity_timer();
                                    let _ = manager.event_tx.send(HardwareEvent::KnobRotated {
                                        knob_id: encoder_id,
                                        delta,
                                    });
                                }
                            }

                            // T056: Page buttons
                            Event::ButtonDown(button_id) if button_id >= 6 => {
                                debug!("Event: PageButtonDown(id={}), sleeping={}", button_id, was_sleeping);
                                if was_sleeping {
                                    // Wake up and drop this event
                                    info!("🔔 Device waking from sleep - dropping page button press event (id={})", button_id);
                                    manager.wake();
                                } else {
                                    // Normal operation - send event and reset activity timer
                                    debug!("Processing page button in normal mode - resetting activity timer");
                                    manager.reset_activity_timer();
                                    let _ = manager.event_tx.send(HardwareEvent::PageButtonPressed {
                                        page_id: button_id - 6,
                                    });
                                }
                            }
                            // T056: Button press/release events
                            Event::ButtonDown(button_id) => {
                                debug!("Event: ButtonDown(id={}), sleeping={}", button_id, was_sleeping);
                                if was_sleeping {
                                    // Wake up and drop this event
                                    info!("🔔 Device waking from sleep - dropping button press event (id={})", button_id);
                                    manager.wake();
                                } else {
                                    // Normal operation - send event and reset activity timer
                                    debug!("Processing button press in normal mode - resetting activity timer");
                                    manager.reset_activity_timer();
                                    let _ = manager
                                        .event_tx
                                        .send(HardwareEvent::ButtonPressed { button_id });
                                }
                            }

                            Event::ButtonUp(button_id) => {
                                debug!("Event: ButtonUp(id={}), sleeping={}", button_id, was_sleeping);
                                // Button release events don't wake device or reset timer
                                if !was_sleeping {
                                    let _ = manager
                                        .event_tx
                                        .send(HardwareEvent::ButtonReleased { button_id });
                                } else {
                                    debug!("Ignoring button release while sleeping");
                                }
                            }

                            // T057: Encoder press/release (treat as page buttons)
                            Event::EncoderDown(encoder_id) => {
                                debug!("Event: EncoderDown(id={}), sleeping={}", encoder_id, was_sleeping);
                                if was_sleeping {
                                    info!("🔔 Device waking from sleep - dropping encoder press event (id={})", encoder_id);
                                    manager.wake();
                                } else {
                                    debug!("Processing encoder press in normal mode - resetting activity timer");
                                    manager.reset_activity_timer();
                                }
                            }

                            // Ignore encoder up events for now
                            Event::EncoderUp(encoder_id) => {
                                debug!("Event: EncoderUp(id={}) - ignored", encoder_id);
                            }
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
        debug!("Hardware display loop iteration");

        // Wait for display command
        if let Ok(()) = command_rx.changed().await {
            // Get the latest command (may have been updated multiple times)
            // Clone to drop the borrow guard before awaiting
            let command = command_rx.borrow_and_update().clone();
            debug!("Received display command: {:?}", command);

            if let Some(command) = command {
                // Lock manager to access device
                let manager_guard = manager.lock().await;
                if let Some(device) = manager_guard.device() {
                    match command {
                        HardwareCommand::UpdateStatusPage(layout) => {
                            debug!("Updating status page");
                            if let Err(e) =
                                display_manager.render_status_page(device, &layout).await
                            {
                                error!("Failed to update status page: {}", e);
                            }
                        }
                        HardwareCommand::UpdateAmplifierPage(layout) => {
                            debug!("Updating amplifier control page");
                            if let Err(e) = display_manager
                                .render_amplifier_control_page(device, &layout)
                                .await
                            {
                                error!("Failed to update amplifier control page: {}", e);
                            }
                        }
                        HardwareCommand::UpdateUnifiedSourceSelectionPage(layout) => {
                            debug!("Updating unified source selection page");
                            if let Err(e) = display_manager
                                .render_unified_source_view(device, &layout)
                                .await
                            {
                                error!("Failed to update unified source selection page: {}", e);
                            }
                        }
                        HardwareCommand::ShowError(error_msg) => {
                            info!("Showing error on display: {}", error_msg);
                            if let Err(e) = display_manager.render_error(device, &error_msg).await {
                                error!("Failed to show error on display: {}", e);
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
    // Spawn sleep check loop as a separate parallel task
    let sleep_manager = manager.clone();
    tokio::spawn(async move {
        DeviceConnectionHandler::sleep_check_loop(sleep_manager).await;
    });

    let mut handler = DeviceConnectionHandler::new(manager, poll_interval);

    handler.handle_connection_lifecycle().await
}
