// Hardware display - Screen rendering for USB HID controller

use crate::hardware::HardwareError;
use ab_glyph::{FontRef, PxScale};
use ajazz_sdk::AsyncAjazz;
use imageproc::drawing::{draw_text_mut, text_size};
use imageproc::image::{DynamicImage, ImageBuffer, Rgba, RgbaImage};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use tracing::debug;

/// Screen dimensions for button displays (typical for Ajazz devices)
const BUTTON_SCREEN_WIDTH: u32 = 80;
const BUTTON_SCREEN_HEIGHT: u32 = 80;

/// Default font size for text rendering
const DEFAULT_FONT_SIZE: f32 = 16.0;

/// Default text color (white)
const TEXT_COLOR: Rgba<u8> = Rgba([255, 255, 255, 255]);

/// Default background color (black)
const BG_COLOR: Rgba<u8> = Rgba([0, 0, 0, 255]);

/// T053: Delay between individual screen updates to prevent USB bandwidth saturation (milliseconds)
const SCREEN_UPDATE_DELAY_MS: u64 = 5;

/// Display manager for rendering to hardware screens
pub struct DisplayManager {
    /// Embedded font for text rendering
    font: FontRef<'static>,
}

impl DisplayManager {
    /// Create a new display manager
    pub fn new() -> Result<Self, HardwareError> {
        // Use embedded DejaVu Sans font
        let font_data = include_bytes!("../../assets/DejaVuSans.ttf");
        let font = FontRef::try_from_slice(font_data)
            .map_err(|_| HardwareError::WriteError("Failed to load font".to_string()))?;

        Ok(Self { font })
    }

    /// Create a blank screen image
    fn create_blank_image(&self) -> RgbaImage {
        ImageBuffer::from_pixel(BUTTON_SCREEN_WIDTH, BUTTON_SCREEN_HEIGHT, BG_COLOR)
    }

    /// Helper: Draw a label at the top of an image
    fn draw_top_label(&self, image: &mut RgbaImage, label: &str, font_size: f32, y_offset: i32) {
        let scale = PxScale::from(font_size);
        let (width, _) = text_size(scale, &self.font, label);
        let x = ((BUTTON_SCREEN_WIDTH as i32 - width as i32) / 2).max(0);
        draw_text_mut(image, TEXT_COLOR, x, y_offset, scale, &self.font, label);
    }

    /// Helper: Draw centered text on an image
    fn draw_centered_text(&self, image: &mut RgbaImage, text: &str, font_size: f32, y_offset: i32) {
        let scale = PxScale::from(font_size);
        let (width, height) = text_size(scale, &self.font, text);
        let x = ((BUTTON_SCREEN_WIDTH as i32 - width as i32) / 2).max(0);
        let y = ((BUTTON_SCREEN_HEIGHT as i32 - height as i32) / 2 + y_offset).max(y_offset.max(0));
        draw_text_mut(image, TEXT_COLOR, x, y, scale, &self.font, text);
    }

    /// Helper: Send an image to a button screen
    async fn send_image_to_button(
        &self,
        device: &Arc<AsyncAjazz>,
        button: u8,
        image: RgbaImage,
    ) -> Result<(), HardwareError> {
        let dynamic_image = DynamicImage::ImageRgba8(image);
        device
            .set_button_image(button, dynamic_image)
            .await
            .map_err(|e| HardwareError::WriteError(e.to_string()))?;

        device
            .flush()
            .await
            .map_err(|e| HardwareError::WriteError(e.to_string()))?;

        Ok(())
    }

    /// Render text centered on a blank screen
    pub fn render_text_centered(&self, text: &str, font_size: f32) -> RgbaImage {
        let mut image = self.create_blank_image();
        let scale = PxScale::from(font_size);

        // Calculate text dimensions
        let (text_width, text_height) = text_size(scale, &self.font, text);

        // Center the text
        let x = ((BUTTON_SCREEN_WIDTH as i32 - text_width as i32) / 2).max(0);
        let y = ((BUTTON_SCREEN_HEIGHT as i32 - text_height as i32) / 2).max(0);

        // Draw text
        draw_text_mut(&mut image, TEXT_COLOR, x, y, scale, &self.font, text);

        image
    }

    /// Render multiline text centered on a blank screen
    pub fn render_multiline_text(&self, lines: &[&str], font_size: f32) -> RgbaImage {
        let mut image = self.create_blank_image();
        let scale = PxScale::from(font_size);

        let line_height = font_size as i32 + 4;
        let total_height = line_height * lines.len() as i32;
        let start_y = ((BUTTON_SCREEN_HEIGHT as i32 - total_height) / 2).max(0);

        for (i, line) in lines.iter().enumerate() {
            let (text_width, _) = text_size(scale, &self.font, line);
            let x = ((BUTTON_SCREEN_WIDTH as i32 - text_width as i32) / 2).max(0);
            let y = start_y + (i as i32 * line_height);

            draw_text_mut(&mut image, TEXT_COLOR, x, y, scale, &self.font, line);
        }

        image
    }

    /// Display a status message on a specific button screen
    pub async fn display_status(
        &self,
        device: &Arc<AsyncAjazz>,
        button: u8,
        message: &str,
    ) -> Result<(), HardwareError> {
        let image = self.render_text_centered(message, DEFAULT_FONT_SIZE);
        let dynamic_image = DynamicImage::ImageRgba8(image);

        device
            .set_button_image(button, dynamic_image)
            .await
            .map_err(|e| HardwareError::WriteError(e.to_string()))?;

        device
            .flush()
            .await
            .map_err(|e| HardwareError::WriteError(e.to_string()))?;

        Ok(())
    }

    /// Display multiline status on a specific button screen
    pub async fn display_multiline_status(
        &self,
        device: &Arc<AsyncAjazz>,
        button: u8,
        lines: &[&str],
        font_size: f32,
    ) -> Result<(), HardwareError> {
        let image = self.render_multiline_text(lines, font_size);
        let dynamic_image = DynamicImage::ImageRgba8(image);

        device
            .set_button_image(button, dynamic_image)
            .await
            .map_err(|e| HardwareError::WriteError(e.to_string()))?;

        device
            .flush()
            .await
            .map_err(|e| HardwareError::WriteError(e.to_string()))?;

        Ok(())
    }

    /// Clear all button screens
    pub async fn clear_all_screens(&self, device: &Arc<AsyncAjazz>) -> Result<(), HardwareError> {
        device
            .clear_all_button_images()
            .await
            .map_err(|e| HardwareError::WriteError(e.to_string()))?;

        device
            .flush()
            .await
            .map_err(|e| HardwareError::WriteError(e.to_string()))?;

        Ok(())
    }

    /// Clear a specific button screen
    pub async fn clear_screen(
        &self,
        device: &Arc<AsyncAjazz>,
        button: u8,
    ) -> Result<(), HardwareError> {
        device
            .clear_button_image(button)
            .await
            .map_err(|e| HardwareError::WriteError(e.to_string()))?;

        device
            .flush()
            .await
            .map_err(|e| HardwareError::WriteError(e.to_string()))?;

        Ok(())
    }
}

impl Default for DisplayManager {
    fn default() -> Self {
        Self::new().expect("Failed to create DisplayManager")
    }
}

/// Connection status display helpers
impl DisplayManager {
    /// Display "Waiting for hardware..." message on all button screens (T028)
    pub async fn show_waiting_for_hardware(
        &self,
        device: &Arc<AsyncAjazz>,
    ) -> Result<(), HardwareError> {
        // Display on button 0 (main status screen)
        self.display_multiline_status(device, 0, &["Waiting for", "hardware..."], 14.0)
            .await
    }

    /// Display "Connecting to server..." message during connection attempt (T029)
    pub async fn show_connecting_to_server(
        &self,
        device: &Arc<AsyncAjazz>,
        server_address: &str,
    ) -> Result<(), HardwareError> {
        // Display on button 0 (main status screen)
        self.display_multiline_status(
            device,
            0,
            &["Connecting", "to server...", server_address],
            12.0,
        )
        .await
    }

    /// Display connection error message on hardware screens (T030)
    pub async fn show_connection_error(
        &self,
        device: &Arc<AsyncAjazz>,
        error_type: &str,
    ) -> Result<(), HardwareError> {
        // Display on button 0 (main status screen)
        self.display_multiline_status(device, 0, &["Connection", "Error:", error_type], 12.0)
            .await
    }

    /// Display room name and connection success on hardware screens (T031)
    pub async fn show_connection_success(
        &self,
        device: &Arc<AsyncAjazz>,
        room_name: &str,
    ) -> Result<(), HardwareError> {
        // Display on button 0 (main status screen)
        self.display_multiline_status(device, 0, &["Connected!", room_name], 14.0)
            .await
    }

    /// Display full status layout with room info
    pub async fn show_status_layout(
        &self,
        device: &Arc<AsyncAjazz>,
        room_name: &str,
        server: &str,
    ) -> Result<(), HardwareError> {
        // Button 0: Mute status (placeholder)
        self.display_status(device, 0, "Mute").await?;

        // Button 1: Stream name (placeholder)
        self.display_status(device, 1, "Stream").await?;

        // Button 2: Volume (placeholder)
        self.display_status(device, 2, "Volume").await?;

        // Button 3: Connection status
        self.display_multiline_status(device, 3, &["Connected"], 14.0)
            .await?;

        // Button 4: Server address
        self.display_multiline_status(device, 4, &["Server:", server], 10.0)
            .await?;

        // Button 5: Room name
        self.display_multiline_status(device, 5, &["Room:", room_name], 10.0)
            .await?;

        Ok(())
    }
}

/// Amplifier control page display helpers
impl DisplayManager {
    /// T028: Render power button showing amplifier power state
    pub async fn render_power_button(
        &self,
        device: &Arc<AsyncAjazz>,
        button: u8,
        power_on: Option<bool>,
    ) -> Result<(), HardwareError> {
        let (_text, lines) = match power_on {
            Some(true) => ("ON", vec!["Power", "ON"]),
            Some(false) => ("OFF", vec!["Power", "OFF"]),
            None => ("?", vec!["Power", "Unknown"]),
        };

        self.display_multiline_status(device, button, &lines, 14.0)
            .await
    }

    /// T029: Render amplifier control page with all button layouts
    pub async fn render_amplifier_control_page(
        &self,
        device: &Arc<AsyncAjazz>,
        layout: &AmplifierControlPageLayout,
    ) -> Result<(), HardwareError> {
        debug!("Rendering amplifier control page: {:?}", layout);

        // Button 0: Power status (T028)
        let power_on = match layout.power_status.as_str() {
            "ON" => Some(true),
            "OFF" => Some(false),
            _ => None,
        };
        self.render_power_button(device, 0, power_on).await?;
        sleep(Duration::from_millis(SCREEN_UPDATE_DELAY_MS)).await;

        // Button 1: Connection status (T037)
        let connected = layout.connection_status == "Connected";
        self.render_connection_screen(device, 1, connected).await?;
        sleep(Duration::from_millis(SCREEN_UPDATE_DELAY_MS)).await;

        // Button 2: Selected source
        self.display_multiline_status(device, 2, &["Source:", &layout.selected_source], 12.0)
            .await?;
        sleep(Duration::from_millis(SCREEN_UPDATE_DELAY_MS)).await;

        // Button 3: Error or status (T040)
        if let Some(ref error) = layout.error_message {
            self.display_multiline_status(device, 3, &["Error:", error], 10.0)
                .await?;
        } else {
            self.display_status(device, 3, "OK").await?;
        }
        sleep(Duration::from_millis(SCREEN_UPDATE_DELAY_MS)).await;

        // Button 4: Info text
        self.display_status(device, 4, &layout.info_text).await?;
        sleep(Duration::from_millis(SCREEN_UPDATE_DELAY_MS)).await;

        // Button 5: Page indicator
        self.display_status(device, 5, &layout.page_indicator)
            .await?;

        Ok(())
    }

    /// T042: Render a source button showing source name and selection indicator
    pub async fn render_source_button(
        &self,
        device: &Arc<AsyncAjazz>,
        button: u8,
        source: crate::homeassistant::AmplifierSource,
        is_selected: bool,
    ) -> Result<(), HardwareError> {
        let indicator = if is_selected { "✓" } else { "" };
        let lines = vec![source.display_name(), indicator];

        self.display_multiline_status(device, button, &lines, 12.0)
            .await
    }

    /// T043: Render source selection page showing all 5 sources
    pub async fn render_source_selection_page(
        &self,
        device: &Arc<AsyncAjazz>,
        layout: &SourceSelectionPageLayout,
    ) -> Result<(), HardwareError> {
        debug!("Rendering source selection page: {:?}", layout);

        // Get all available sources
        use crate::homeassistant::AmplifierSource;
        let sources = [
            AmplifierSource::Phono,
            AmplifierSource::CD,
            AmplifierSource::Spotify,
            AmplifierSource::Source4,
            AmplifierSource::Source5,
        ];

        // Render buttons 0-4 with sources
        for (i, source) in sources.iter().enumerate() {
            let is_selected = *source == layout.selected_source;
            self.render_source_button(device, i as u8, *source, is_selected)
                .await?;
            sleep(Duration::from_millis(SCREEN_UPDATE_DELAY_MS)).await;
        }

        // Button 5: Page indicator
        self.display_status(device, 5, &layout.page_indicator)
            .await?;

        Ok(())
    }
}

/// Status page layout data
/// T042: Screen layout for status page displaying room state across 6 button screens
#[derive(Debug, Clone)]
pub struct StatusPageLayout {
    /// Mute status display text (button 0)
    pub mute_status: String,

    /// Current stream name (button 1)
    pub stream_name: String,

    /// Volume percentage display (button 2)
    pub volume_display: String,

    /// Connection status display (button 3)
    pub connection_status: String,

    /// Server address display (button 4)
    pub server_address: String,

    /// Room name display (button 5) - also serves as page indicator
    pub room_name: String,

    /// T066: Page indicator text
    pub page_indicator: String,
}

impl StatusPageLayout {
    /// Create a new status page layout from room state
    /// T042: Build status page layout data structure
    pub fn from_room_state(
        room_name: &str,
        server_address: &str,
        volume: u8,
        muted: bool,
        connected: bool,
        stream_name: Option<&str>,
    ) -> Self {
        // Button 0: Mute status
        let mute_status = if muted {
            "MUTED".to_string()
        } else {
            "Unmuted".to_string()
        };

        // Button 1: Current stream name
        let stream_name = stream_name.unwrap_or("No Stream").to_string();

        // Button 2: Volume percentage
        let volume_display = format!("{}%", volume);

        // Button 3: Connection status
        let connection_status = if connected {
            "Connected".to_string()
        } else {
            "Disconnected".to_string()
        };

        // Button 4: Server address
        let server_address = server_address.to_string();

        // Button 5: Room name
        let room_name = room_name.to_string();

        Self {
            mute_status,
            stream_name,
            volume_display,
            connection_status,
            server_address,
            room_name,
            page_indicator: "Status".to_string(),
        }
    }
}

/// Status page rendering
/// T042: Render complete status page layout to all 6 button screens
impl DisplayManager {
    /// Render the complete status page layout to all button screens
    /// T042: Display status page across 6 button screens
    /// T053: Includes batching delays to prevent USB bandwidth saturation
    pub async fn render_status_page(
        &self,
        device: &Arc<AsyncAjazz>,
        layout: &StatusPageLayout,
    ) -> Result<(), HardwareError> {
        debug!("Rendering status page: {:?}", layout);
        // Button 0: Mute status (T044)
        let muted = layout.mute_status == "MUTED";
        self.render_mute_screen(device, 0, muted).await?;
        // sleep(Duration::from_millis(SCREEN_UPDATE_DELAY_MS)).await;

        // Button 1: Stream name (T045)
        self.render_stream_screen(device, 1, &layout.stream_name)
            .await?;
        // sleep(Duration::from_millis(SCREEN_UPDATE_DELAY_MS)).await;

        // Button 2: Volume percentage (T043)
        self.render_volume_screen(
            device,
            2,
            layout
                .volume_display
                .trim_end_matches('%')
                .parse()
                .unwrap_or(0),
        )
        .await?;
        // sleep(Duration::from_millis(SCREEN_UPDATE_DELAY_MS)).await;

        // Button 3: Connection status (T046)
        let connected = layout.connection_status == "Connected";
        self.render_connection_screen(device, 3, connected).await?;
        // sleep(Duration::from_millis(SCREEN_UPDATE_DELAY_MS)).await;

        // Button 4: Server address (T047)
        self.render_server_screen(device, 4, &layout.server_address)
            .await?;
        // sleep(Duration::from_millis(SCREEN_UPDATE_DELAY_MS)).await;

        // Button 5: Room name (T048) with page indicator (T066)
        self.render_room_screen(device, 5, &layout.room_name, &layout.page_indicator)
            .await?;

        Ok(())
    }

    /// Render volume percentage on button screen 2
    /// T043: Display volume with large percentage text and label
    pub async fn render_volume_screen(
        &self,
        device: &Arc<AsyncAjazz>,
        button: u8,
        volume: u8,
    ) -> Result<(), HardwareError> {
        let mut image = self.create_blank_image();
        self.draw_top_label(&mut image, "VOLUME", 10.0, 5);
        self.draw_centered_text(&mut image, &format!("{}%", volume), 24.0, 5);
        self.send_image_to_button(device, button, image).await
    }

    /// Render mute status on button screen 0
    /// T044: Display mute status with clear visual indication
    pub async fn render_mute_screen(
        &self,
        device: &Arc<AsyncAjazz>,
        button: u8,
        muted: bool,
    ) -> Result<(), HardwareError> {
        let mut image = self.create_blank_image();
        self.draw_top_label(&mut image, "MUTE", 12.0, 8);
        self.draw_centered_text(&mut image, if muted { "ON" } else { "OFF" }, 22.0, 5);
        self.send_image_to_button(device, button, image).await
    }

    /// Render current stream name on button screen 1
    /// T045: Display stream name with label
    pub async fn render_stream_screen(
        &self,
        device: &Arc<AsyncAjazz>,
        button: u8,
        stream_name: &str,
    ) -> Result<(), HardwareError> {
        let mut image = self.create_blank_image();
        self.draw_top_label(&mut image, "STREAM", 10.0, 5);

        // Truncate stream name if too long
        let display_name = if stream_name.len() > 12 {
            format!("{}...", &stream_name[0..9])
        } else {
            stream_name.to_string()
        };

        self.draw_centered_text(&mut image, &display_name, 14.0, 5);
        self.send_image_to_button(device, button, image).await
    }

    /// Render connection status on button screen 3
    /// T046: Display connection status with visual indicator
    pub async fn render_connection_screen(
        &self,
        device: &Arc<AsyncAjazz>,
        button: u8,
        connected: bool,
    ) -> Result<(), HardwareError> {
        let mut image = self.create_blank_image();
        self.draw_top_label(&mut image, "STATUS", 10.0, 5);
        self.draw_centered_text(&mut image, if connected { "OK" } else { "DISC" }, 20.0, 5);
        self.send_image_to_button(device, button, image).await
    }

    /// Render server address on button screen 4
    /// T047: Display server address with label
    pub async fn render_server_screen(
        &self,
        device: &Arc<AsyncAjazz>,
        button: u8,
        server_address: &str,
    ) -> Result<(), HardwareError> {
        let mut image = self.create_blank_image();
        self.draw_top_label(&mut image, "SERVER", 10.0, 5);

        // Split server address into multiple lines if needed (IP:port format)
        if server_address.contains(':') {
            let parts: Vec<&str> = server_address.split(':').collect();
            let scale = PxScale::from(11.0);
            let line_height = 14;
            let start_y = 28;

            for (i, part) in parts.iter().enumerate() {
                let (width, _) = text_size(scale, &self.font, part);
                let x = ((BUTTON_SCREEN_WIDTH as i32 - width as i32) / 2).max(0);
                let y = start_y + (i as i32 * line_height);
                draw_text_mut(&mut image, TEXT_COLOR, x, y, scale, &self.font, part);
            }
        } else {
            self.draw_centered_text(&mut image, server_address, 11.0, 5);
        }

        self.send_image_to_button(device, button, image).await
    }

    /// Render room name on button screen 5
    /// T048: Display room name with label
    /// T066: Also displays page indicator at bottom
    pub async fn render_room_screen(
        &self,
        device: &Arc<AsyncAjazz>,
        button: u8,
        room_name: &str,
        page_indicator: &str,
    ) -> Result<(), HardwareError> {
        let mut image = self.create_blank_image();
        self.draw_top_label(&mut image, "ROOM", 12.0, 8);

        // Truncate room name if too long
        let display_name = if room_name.len() > 12 {
            format!("{}...", &room_name[0..9])
        } else {
            room_name.to_string()
        };

        self.draw_centered_text(&mut image, &display_name, 14.0, -5);

        // T066: Draw page indicator at bottom
        self.draw_top_label(&mut image, page_indicator, 10.0, 60);

        self.send_image_to_button(device, button, image).await
    }

    /// Render stream selection screen for a single button
    /// T069: Display stream name on button with selection indicator
    pub async fn render_stream_button(
        &self,
        device: &Arc<AsyncAjazz>,
        button: u8,
        stream_name: Option<&str>,
        is_selected: bool,
    ) -> Result<(), HardwareError> {
        let mut image = self.create_blank_image();

        if let Some(name) = stream_name {
            // Show selection indicator
            if is_selected {
                self.draw_top_label(&mut image, ">", 16.0, 2);
            }

            // Truncate stream name if too long
            let display_name = if name.len() > 10 {
                format!("{}...", &name[0..7])
            } else {
                name.to_string()
            };

            self.draw_centered_text(&mut image, &display_name, 13.0, 5);
        } else {
            // Empty slot
            self.draw_centered_text(&mut image, "---", 14.0, 0);
        }

        self.send_image_to_button(device, button, image).await
    }

    /// Render complete stream selection page layout to all button screens
    /// T069: Display available streams across 6 button screens
    /// T066: Include page indicator
    pub async fn render_stream_selection_page(
        &self,
        device: &Arc<AsyncAjazz>,
        layout: &StreamSelectionPageLayout,
    ) -> Result<(), HardwareError> {
        // Render buttons 0-4 with stream names
        for button in 0..5 {
            self.render_stream_button(
                device,
                button,
                layout.stream_names[button as usize].as_deref(),
                layout.selected_button == Some(button),
            )
            .await?;
        }

        // T066: Button 5 shows stream (if available) with page indicator at bottom
        let mut image = self.create_blank_image();

        // Show stream name if available
        if let Some(ref stream_name) = layout.stream_names[5] {
            if layout.selected_button == Some(5) {
                self.draw_top_label(&mut image, ">", 16.0, 2);
            }

            let display_name = if stream_name.len() > 10 {
                format!("{}...", &stream_name[0..7])
            } else {
                stream_name.to_string()
            };

            self.draw_centered_text(&mut image, &display_name, 13.0, -5);
        }

        // Draw page indicator at bottom
        self.draw_top_label(&mut image, &layout.page_indicator, 10.0, 60);

        self.send_image_to_button(device, 5, image).await?;

        Ok(())
    }

    /// Render error message on screens
    /// T073: Display error message when command fails
    pub async fn render_error(
        &self,
        device: &Arc<AsyncAjazz>,
        error_message: &str,
    ) -> Result<(), HardwareError> {
        // Display error on all button screens
        for button in 0..6 {
            let mut image = self.create_blank_image();
            self.draw_top_label(&mut image, "ERROR", 12.0, 5);

            // Split error message into words that fit
            let words: Vec<&str> = error_message.split_whitespace().collect();
            let max_chars = 10;
            let mut lines = Vec::new();
            let mut current_line = String::new();

            for word in words {
                if current_line.len() + word.len() < max_chars {
                    if !current_line.is_empty() {
                        current_line.push(' ');
                    }
                    current_line.push_str(word);
                } else {
                    if !current_line.is_empty() {
                        lines.push(current_line);
                    }
                    current_line = word.to_string();
                }
            }
            if !current_line.is_empty() {
                lines.push(current_line);
            }

            // Draw up to 3 lines
            let line_height = 12;
            let start_y = 30;
            for (i, line) in lines.iter().take(3).enumerate() {
                let scale = PxScale::from(10.0);
                let (width, _) = text_size(scale, &self.font, line);
                let x = ((BUTTON_SCREEN_WIDTH as i32 - width as i32) / 2).max(0);
                let y = start_y + (i as i32 * line_height);
                draw_text_mut(&mut image, TEXT_COLOR, x, y, scale, &self.font, line);
            }

            self.send_image_to_button(device, button, image).await?;
        }

        Ok(())
    }
}

/// Amplifier control page layout data
/// T027: Screen layout for amplifier control page
#[derive(Debug, Clone)]
pub struct AmplifierControlPageLayout {
    /// Amplifier power status (button 0)
    pub power_status: String,

    /// Connection status to Home Assistant (button 1)
    pub connection_status: String,

    /// Currently selected source (button 2)
    pub selected_source: String,

    /// Error message if any (button 3)
    pub error_message: Option<String>,

    /// Info/help text (button 4)
    pub info_text: String,

    /// Page indicator (button 5)
    pub page_indicator: String,
}

impl AmplifierControlPageLayout {
    /// Create a new amplifier control page layout
    pub fn new(
        power_on: Option<bool>,
        connected: bool,
        selected_source: &str,
        error: Option<String>,
    ) -> Self {
        let power_status = match power_on {
            Some(true) => "ON".to_string(),
            Some(false) => "OFF".to_string(),
            None => "Unknown".to_string(),
        };

        let connection_status = if connected {
            "Connected".to_string()
        } else {
            "Disconnected".to_string()
        };

        Self {
            power_status,
            connection_status,
            selected_source: selected_source.to_string(),
            error_message: error,
            info_text: "Amplifier".to_string(),
            page_indicator: "Amp".to_string(),
        }
    }
}

/// Source selection page layout data
/// T041: Layout for amplifier source selection page
#[derive(Debug, Clone)]
pub struct SourceSelectionPageLayout {
    /// Currently selected source (to highlight)
    pub selected_source: crate::homeassistant::AmplifierSource,

    /// Page indicator text
    pub page_indicator: String,
}

impl SourceSelectionPageLayout {
    /// T041: Create a new source selection page layout
    pub fn new(selected_source: crate::homeassistant::AmplifierSource) -> Self {
        Self {
            selected_source,
            page_indicator: "Sources".to_string(),
        }
    }
}

/// Stream selection page layout data
/// T069: Layout for stream selection page showing available streams
#[derive(Debug, Clone)]
pub struct StreamSelectionPageLayout {
    /// Stream names for buttons 0-5 (None if no stream at that position)
    pub stream_names: [Option<String>; 6],

    /// Which button is currently selected (None if no selection)
    pub selected_button: Option<u8>,

    /// T066: Page indicator text
    pub page_indicator: String,
}

impl StreamSelectionPageLayout {
    /// Create a new stream selection page layout from available streams
    /// T069: Build stream selection page layout data structure
    pub fn from_streams(
        streams: &[crate::snapcast::types::AudioStream],
        selected_index: usize,
    ) -> Self {
        let mut stream_names: [Option<String>; 6] = Default::default();

        // Fill in up to 6 streams
        for (i, stream) in streams.iter().take(6).enumerate() {
            stream_names[i] = Some(stream.name.clone());
        }

        // Determine selected button (if selected_index is within displayed range)
        let selected_button = if selected_index < 6 {
            Some(selected_index as u8)
        } else {
            None
        };

        Self {
            stream_names,
            selected_button,
            page_indicator: "Streams".to_string(),
        }
    }
}
