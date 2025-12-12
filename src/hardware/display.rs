// Hardware display - Screen rendering for USB HID controller

use crate::hardware::HardwareError;
use ab_glyph::{FontRef, PxScale};
use ajazz_sdk::AsyncAjazz;
use imageproc::drawing::{draw_text_mut, text_size};
use imageproc::image::{DynamicImage, ImageBuffer, Rgba, RgbaImage};
use std::sync::Arc;

/// Screen dimensions for button displays (typical for Ajazz devices)
const BUTTON_SCREEN_WIDTH: u32 = 80;
const BUTTON_SCREEN_HEIGHT: u32 = 80;

/// Default font size for text rendering
const DEFAULT_FONT_SIZE: f32 = 16.0;

/// Default text color (white)
const TEXT_COLOR: Rgba<u8> = Rgba([255, 255, 255, 255]);

/// Default background color (black)
const BG_COLOR: Rgba<u8> = Rgba([0, 0, 0, 255]);

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
        self.display_multiline_status(
            device,
            0,
            &["Waiting for", "hardware..."],
            14.0,
        )
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
        self.display_multiline_status(
            device,
            0,
            &["Connection", "Error:", error_type],
            12.0,
        )
        .await
    }

    /// Display room name and connection success on hardware screens (T031)
    pub async fn show_connection_success(
        &self,
        device: &Arc<AsyncAjazz>,
        room_name: &str,
    ) -> Result<(), HardwareError> {
        // Display on button 0 (main status screen)
        self.display_multiline_status(
            device,
            0,
            &["Connected!", room_name],
            14.0,
        )
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
