use std::time::Duration;

use rumqttc::{AsyncClient, Event, EventLoop, MqttOptions, Packet, QoS};
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

use crate::config::settings::{AmplifierPowerEntityConfig, HomeAssistantConfig};

use super::commands;
use super::types::{
    AmplifierState, HomeAssistantCommand, HomeAssistantConnection, HomeAssistantEvent,
    Zigbee2MqttStatePayload,
};

/// MQTT client for Home Assistant integration and IR Blaster control
pub struct MqttClient {
    /// MQTT client for publishing
    client: AsyncClient,

    /// Connection state
    connection: HomeAssistantConnection,

    /// Power entity configuration
    power_entity_config: AmplifierPowerEntityConfig,

    /// IR Blaster MQTT topic
    ir_blaster_topic: String,

    /// Current amplifier state (for toggle operations and tracking power)
    amplifier_state: AmplifierState,

    /// Channel to send events to main application
    event_tx: mpsc::UnboundedSender<HomeAssistantEvent>,

    /// Channel to receive commands from main application
    command_rx: mpsc::UnboundedReceiver<HomeAssistantCommand>,

    /// T058: Timestamp of last volume command sent (for rate limiting)
    last_volume_command: Option<std::time::Instant>,
}

impl MqttClient {
    /// Create a new MQTT client
    pub fn new(
        config: HomeAssistantConfig,
        event_tx: mpsc::UnboundedSender<HomeAssistantEvent>,
        command_rx: mpsc::UnboundedReceiver<HomeAssistantCommand>,
    ) -> (Self, EventLoop) {
        let connection = HomeAssistantConnection::new(
            config.broker_address.clone(),
            config.broker_port,
            config.username.clone(),
        );

        // Configure MQTT options
        let mut mqtt_options = MqttOptions::new(
            connection.client_id.clone(),
            config.broker_address.clone(),
            config.broker_port,
        );

        // Set authentication if provided
        if let (Some(username), Some(password)) = (config.username.clone(), config.password) {
            mqtt_options.set_credentials(username, password);
        }

        // Set connection parameters per MQTT API contract
        mqtt_options.set_keep_alive(Duration::from_secs(60));
        mqtt_options.set_clean_session(true);

        let (client, eventloop) = AsyncClient::new(mqtt_options, 10);

        let mqtt_client = Self {
            client,
            connection,
            power_entity_config: config.amplifier,
            ir_blaster_topic: config.ir_blaster_topic,
            amplifier_state: AmplifierState::new(),
            event_tx,
            command_rx,
            last_volume_command: None,
        };

        (mqtt_client, eventloop)
    }

    /// Main event loop for MQTT client
    pub async fn run(mut self, mut eventloop: EventLoop) {
        loop {
            tokio::select! {
                // Handle MQTT events from broker
                event = eventloop.poll() => {
                    match event {
                        Ok(event) => {
                            if let Err(e) = self.handle_mqtt_event(event).await {
                                error!("Error handling MQTT event: {}", e);
                            }
                        }
                        Err(e) => {
                            error!("MQTT connection error: {}", e);
                            self.connection.set_disconnected();
                            let _ = self.event_tx.send(HomeAssistantEvent::BrokerDisconnected);

                            // Exponential backoff reconnection
                            self.connection.increase_backoff();
                            info!("Reconnecting in {:?}", self.connection.reconnect_delay);
                            tokio::time::sleep(self.connection.reconnect_delay).await;
                        }
                    }
                }

                // Handle commands from application
                Some(command) = self.command_rx.recv() => {
                    if let Err(e) = self.handle_command(command).await {
                        error!("Error handling command: {}", e);
                    }
                }
            }
        }
    }

    /// Handle MQTT events from broker
    async fn handle_mqtt_event(&mut self, event: Event) -> Result<(), Box<dyn std::error::Error>> {
        match event {
            Event::Incoming(Packet::ConnAck(_)) => {
                info!("Connected to MQTT broker");
                self.connection.set_connected();
                let _ = self.event_tx.send(HomeAssistantEvent::BrokerConnected);

                // Subscribe to entity state topics
                self.subscribe_to_entities().await?;
            }

            Event::Incoming(Packet::Publish(publish)) => {
                self.handle_publish(publish).await?;
            }

            Event::Incoming(Packet::Disconnect) => {
                warn!("Disconnected from MQTT broker");
                self.connection.set_disconnected();
                let _ = self.event_tx.send(HomeAssistantEvent::BrokerDisconnected);
            }

            _ => {
                // Ignore other packet types
                debug!("Received MQTT event: {:?}", event);
            }
        }

        Ok(())
    }

    /// Subscribe to power entity state topic
    /// Note: Source selection is managed locally, not via Home Assistant
    async fn subscribe_to_entities(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Extract object ID from power entity ID
        let power_object_id = self
            .power_entity_config
            .power_entity
            .split('.')
            .nth(1)
            .ok_or("Invalid power entity ID")?;

        // Subscribe to power state topic
        let power_state_topic = format!("zigbee2mqtt/{}", power_object_id);
        self.client
            .subscribe(&power_state_topic, QoS::AtLeastOnce)
            .await?;
        info!("Subscribed to power state: {}", power_state_topic);

        // Subscribe to power availability topic
        let power_avail_topic = format!("homeassistant/switch/{}/availability", power_object_id);
        self.client
            .subscribe(&power_avail_topic, QoS::AtLeastOnce)
            .await?;
        info!("Subscribed to power availability: {}", power_avail_topic);

        Ok(())
    }

    /// Handle incoming MQTT publish messages
    async fn handle_publish(
        &mut self,
        publish: rumqttc::Publish,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let topic = publish.topic;
        let payload = String::from_utf8_lossy(&publish.payload).to_string();

        // Determine which entity this update is for
        if topic.contains("zigbee2mqtt") {
            self.handle_state_update(&topic, &payload).await?;
        } else if topic.contains("/availability") {
            self.handle_availability_update(&topic, &payload).await?;
        }

        Ok(())
    }

    /// Handle power entity state updates
    async fn handle_state_update(
        &mut self,
        topic: &str,
        payload: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let power_object_id = self
            .power_entity_config
            .power_entity
            .split('.')
            .nth(1)
            .ok_or("Invalid power entity ID")?;

        // Only handle power state (source is managed locally)
        if topic.contains(power_object_id) {
            // Parse JSON payload from Zigbee2MQTT
            let parsed: Zigbee2MqttStatePayload = match serde_json::from_str(payload) {
                Ok(p) => p,
                Err(e) => {
                    error!("Failed to parse Zigbee2MQTT payload: {}", e);
                    debug!("Invalid payload: {}", payload);
                    return Ok(()); // Skip this message
                }
            };

            // Power state update
            let power_on = parsed.state.to_uppercase() == "ON";
            self.amplifier_state.set_power(power_on);
            info!(
                "Power state changed: {} ({})",
                if power_on { "ON" } else { "OFF" },
                self.power_entity_config.power_entity
            );

            let _ = self.event_tx.send(HomeAssistantEvent::EntityStateChanged {
                entity_id: self.power_entity_config.power_entity.clone(),
                state: parsed.state,
            });
        }

        Ok(())
    }

    /// Handle power entity availability updates
    async fn handle_availability_update(
        &mut self,
        _topic: &str,
        payload: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let available = payload.to_lowercase() == "online";

        info!(
            "Entity availability changed: {} ({})",
            if available { "online" } else { "offline" },
            self.power_entity_config.power_entity
        );

        let _ = self
            .event_tx
            .send(HomeAssistantEvent::EntityAvailabilityChanged {
                entity_id: self.power_entity_config.power_entity.clone(),
                available,
            });

        Ok(())
    }

    /// Handle commands from application
    async fn handle_command(
        &mut self,
        command: HomeAssistantCommand,
    ) -> Result<(), Box<dyn std::error::Error>> {
        match command {
            HomeAssistantCommand::TogglePower => {
                // Determine target state based on current state
                let target_state = match self.amplifier_state.power_on {
                    Some(true) => false,
                    Some(false) => true,
                    None => {
                        // Unknown state, cannot toggle
                        let _ = self.event_tx.send(HomeAssistantEvent::CommandFailed {
                            entity_id: self.power_entity_config.power_entity.clone(),
                            error: "Power state unknown, cannot toggle".to_string(),
                        });
                        return Ok(());
                    }
                };

                self.set_power(target_state).await?;
            }

            HomeAssistantCommand::SetPower { on } => {
                self.set_power(on).await?;
            }

            HomeAssistantCommand::SelectSource { source } => {
                self.send_ir_command_for_source(source).await?;
            }

            HomeAssistantCommand::VolumeUp => {
                self.send_ir_command_for_volume_up().await?;
            }

            HomeAssistantCommand::VolumeDown => {
                self.send_ir_command_for_volume_down().await?;
            }

            HomeAssistantCommand::Disconnect => {
                self.client.disconnect().await?;
            }
        }

        Ok(())
    }

    /// Publish power command to Home Assistant
    async fn set_power(&mut self, on: bool) -> Result<(), Box<dyn std::error::Error>> {
        let power_object_id = self
            .power_entity_config
            .power_entity
            .split('.')
            .nth(1)
            .ok_or("Invalid power entity ID")?;

        let topic = format!("zigbee2mqtt/{}/set", power_object_id);
        let payload = if on { "ON" } else { "OFF" };

        self.client
            .publish(topic.clone(), QoS::AtLeastOnce, false, payload)
            .await?;

        info!("Published power command to {}: {}", topic, payload);

        let _ = self.event_tx.send(HomeAssistantEvent::CommandAcknowledged {
            entity_id: self.power_entity_config.power_entity.clone(),
        });

        Ok(())
    }

    /// Send IR command to select amplifier source
    async fn send_ir_command_for_source(
        &mut self,
        source: super::types::AmplifierSource,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let ir_command = commands::build_source_command(source);
        let payload = ir_command.to_json()?;

        self.client
            .publish(
                self.ir_blaster_topic.clone(),
                QoS::AtLeastOnce,
                false,
                payload.as_bytes(),
            )
            .await?;

        info!(
            "Published IR source command to {}: {}",
            self.ir_blaster_topic,
            source.display_name()
        );
        debug!("IR command payload: {}", payload);

        Ok(())
    }

    /// Send IR command to increase volume
    /// T058: Implements rate limiting (minimum 100ms between commands)
    async fn send_ir_command_for_volume_up(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // T058: Rate limiting - check if enough time has passed since last command
        if let Some(last_time) = self.last_volume_command {
            let elapsed = last_time.elapsed();
            if elapsed < Duration::from_millis(100) {
                warn!(
                    "Rate limiting: Skipping volume command ({}ms since last)",
                    elapsed.as_millis()
                );
                return Ok(());
            }
        }

        let ir_command = commands::build_volume_up_command();
        let payload = ir_command.to_json()?;

        self.client
            .publish(
                self.ir_blaster_topic.clone(),
                QoS::AtLeastOnce,
                false,
                payload.as_bytes(),
            )
            .await?;

        // T058: Update timestamp
        self.last_volume_command = Some(std::time::Instant::now());

        info!(
            "Published IR volume up command to {}",
            self.ir_blaster_topic
        );
        debug!("IR command payload: {}", payload);

        Ok(())
    }

    /// Send IR command to decrease volume
    /// T058: Implements rate limiting (minimum 100ms between commands)
    async fn send_ir_command_for_volume_down(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // T058: Rate limiting - check if enough time has passed since last command
        if let Some(last_time) = self.last_volume_command {
            let elapsed = last_time.elapsed();
            if elapsed < Duration::from_millis(100) {
                warn!(
                    "Rate limiting: Skipping volume command ({}ms since last)",
                    elapsed.as_millis()
                );
                return Ok(());
            }
        }

        let ir_command = commands::build_volume_down_command();
        let payload = ir_command.to_json()?;

        self.client
            .publish(
                self.ir_blaster_topic.clone(),
                QoS::AtLeastOnce,
                false,
                payload.as_bytes(),
            )
            .await?;

        // T058: Update timestamp
        self.last_volume_command = Some(std::time::Instant::now());

        info!(
            "Published IR volume down command to {}",
            self.ir_blaster_topic
        );
        debug!("IR command payload: {}", payload);

        Ok(())
    }
}
