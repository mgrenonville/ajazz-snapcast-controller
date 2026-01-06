use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::time::SystemTime;

use crate::homeassistant::types::AmplifierSource;
use crate::snapcast::types::AudioStream;

/// Unified representation of an audio source (Snapcast stream or amplifier input)
///
/// This enum abstracts the difference between Snapcast streams and direct amplifier inputs,
/// allowing the UI to present a single unified list of all available audio sources.
///
/// # Design Rationale
/// Users don't need to know that Snapcast is technically one of the amplifier's inputs.
/// They just want to select "what to listen to" from a simple list. This abstraction
/// makes that possible by treating both types of sources uniformly in the UI.
///
/// # Usage
/// ```ignore
/// // Snapcast streams are created from the Snapcast server's stream list
/// let stream_source = UnifiedSource::SnapcastStream {
///     stream_id: "living_room".to_string(),
///     name: "Living Room".to_string(),
/// };
///
/// // Amplifier inputs are the direct physical inputs (except the one connected to Snapcast)
/// let phono_source = UnifiedSource::AmplifierInput {
///     source: AmplifierSource::Phono,
///     name: "Phono".to_string(),
/// };
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum UnifiedSource {
    /// Snapcast audio stream
    SnapcastStream {
        /// Unique stream identifier from Snapcast server
        stream_id: String,
        /// Display name (e.g., "Living Room", "Bluetooth")
        name: String,
    },

    /// Direct amplifier input (non-Snapcast)
    ///
    /// Note: The amplifier input that is connected to Snapcast output
    /// should NOT be included here, as Snapcast streams already represent
    /// that input's content.
    AmplifierInput {
        /// Amplifier source enum (Phono, CD, etc.)
        source: AmplifierSource,
        /// Display name (e.g., "Phono", "CD Player")
        name: String,
    },
}

impl UnifiedSource {
    /// Get display name for UI rendering
    pub fn display_name(&self) -> &str {
        match self {
            UnifiedSource::SnapcastStream { name, .. } => name,
            UnifiedSource::AmplifierInput { name, .. } => name,
        }
    }

    /// Check if this is the Snapcast-connected amplifier source
    pub fn is_snapcast_source(&self, config_snapcast_source: AmplifierSource) -> bool {
        match self {
            UnifiedSource::SnapcastStream { .. } => false,
            UnifiedSource::AmplifierInput { source, .. } => *source == config_snapcast_source,
        }
    }

    /// Get a unique identifier for equality comparison
    pub fn identifier(&self) -> String {
        match self {
            UnifiedSource::SnapcastStream { stream_id, .. } => {
                format!("snapcast:{}", stream_id)
            }
            UnifiedSource::AmplifierInput { source, .. } => format!("amplifier:{:?}", source),
        }
    }
}

impl std::fmt::Display for UnifiedSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

/// Collection of all available unified sources
///
/// This struct manages the complete list of selectable audio sources,
/// combining Snapcast streams and non-Snapcast amplifier inputs into
/// a single collection.
///
/// # Responsibilities
/// - Aggregates Snapcast streams and amplifier inputs
/// - Tracks the currently active source
/// - Manages UI selection state for navigation
/// - Handles dynamic updates when Snapcast streams change
///
/// # Important Notes
/// The amplifier input configured as `snapcast_amplifier_source` is excluded
/// from the collection because Snapcast streams already represent that input's content.
/// For example, if Spotify is connected to Snapcast, "Spotify" won't appear as
/// a separate amplifier input - instead, users select from the Snapcast streams.
#[derive(Debug, Clone)]
pub struct UnifiedSourceCollection {
    /// All available sources (Snapcast streams + amplifier inputs)
    sources: Vec<UnifiedSource>,

    /// Currently active source (if any)
    active_source: Option<UnifiedSource>,

    /// Amplifier source configured for Snapcast output
    snapcast_amplifier_source: AmplifierSource,

    /// Source index for UI selection/navigation
    selected_index: usize,
}

impl UnifiedSourceCollection {
    /// Create a new collection from Snapcast streams and amplifier config
    pub fn new(
        snapcast_streams: Vec<AudioStream>,
        snapcast_amplifier_source: AmplifierSource,
    ) -> Self {
        let mut sources = Vec::new();

        // Add Snapcast streams
        for stream in snapcast_streams {
            sources.push(UnifiedSource::SnapcastStream {
                stream_id: stream.stream_id,
                name: stream.name,
            });
        }

        // Add all amplifier sources EXCEPT the one connected to Snapcast
        // (Snapcast streams already represent that source's content)
        for amp_source in AmplifierSource::all() {
            if amp_source != snapcast_amplifier_source {
                sources.push(UnifiedSource::AmplifierInput {
                    source: amp_source,
                    name: amp_source.display_name().to_string(),
                });
            }
        }

        Self {
            sources,
            active_source: None,
            snapcast_amplifier_source,
            selected_index: 0,
        }
    }

    /// Update available Snapcast streams (called on stream list changes)
    pub fn update_snapcast_streams(&mut self, streams: Vec<AudioStream>) {
        // Remove old Snapcast streams
        self.sources
            .retain(|s| !matches!(s, UnifiedSource::SnapcastStream { .. }));

        // Add new Snapcast streams at the beginning
        let mut new_streams: Vec<UnifiedSource> = streams
            .into_iter()
            .map(|stream| UnifiedSource::SnapcastStream {
                stream_id: stream.stream_id,
                name: stream.name,
            })
            .collect();

        new_streams.extend(self.sources.drain(..));
        self.sources = new_streams;

        // Reset selected_index if out of bounds
        if self.selected_index >= self.sources.len() && !self.sources.is_empty() {
            self.selected_index = 0;
        }
    }

    /// Set the currently active source
    pub fn set_active(&mut self, source: UnifiedSource) {
        self.active_source = Some(source);
    }

    /// Get currently active source
    pub fn active_source(&self) -> Option<&UnifiedSource> {
        self.active_source.as_ref()
    }

    /// Get all sources
    pub fn all_sources(&self) -> &[UnifiedSource] {
        &self.sources
    }

    /// Get currently selected source (for UI navigation)
    pub fn selected_source(&self) -> Option<&UnifiedSource> {
        self.sources.get(self.selected_index)
    }

    /// Navigate selection up (wraps around)
    pub fn select_previous(&mut self) {
        if self.sources.is_empty() {
            return;
        }
        if self.selected_index == 0 {
            self.selected_index = self.sources.len() - 1;
        } else {
            self.selected_index -= 1;
        }
    }

    /// Navigate selection down (wraps around)
    pub fn select_next(&mut self) {
        if self.sources.is_empty() {
            return;
        }
        self.selected_index = (self.selected_index + 1) % self.sources.len();
    }

    /// Confirm selection (activate the currently selected source)
    pub fn confirm_selection(&self) -> Option<&UnifiedSource> {
        self.selected_source()
    }
}

/// Context for activating a unified source
///
/// This struct encapsulates the logic for determining what commands need
/// to be sent when activating a source. Different source types require
/// different activation sequences:
///
/// - **Snapcast streams**: Require TWO steps:
///   1. Switch amplifier to Snapcast input
///   2. Select the stream on Snapcast server
///
/// - **Amplifier inputs**: Require ONE step:
///   1. Switch amplifier to the specified input
///
/// # Example
/// ```ignore
/// let context = SourceActivationContext::new(
///     &selected_source,
///     AmplifierSource::Spotify, // Snapcast is connected to Spotify input
/// );
///
/// let (amplifier_source, stream_id) = context.get_activation_commands();
/// // Send commands to amplifier and Snapcast as needed
/// ```
pub struct SourceActivationContext<'a> {
    /// Target source to activate
    target_source: &'a UnifiedSource,

    /// Snapcast amplifier source from configuration
    snapcast_amplifier_source: AmplifierSource,
}

impl<'a> SourceActivationContext<'a> {
    /// Create a new activation context
    pub fn new(
        target_source: &'a UnifiedSource,
        snapcast_amplifier_source: AmplifierSource,
    ) -> Self {
        Self {
            target_source,
            snapcast_amplifier_source,
        }
    }

    /// Get the target source
    pub fn target_source(&self) -> &UnifiedSource {
        self.target_source
    }

    /// Get commands needed to activate this source
    /// Returns (amplifier_source_to_switch_to, optional_snapcast_stream_id)
    pub fn get_activation_commands(&self) -> (AmplifierSource, Option<String>) {
        match self.target_source {
            UnifiedSource::SnapcastStream { stream_id, .. } => {
                // Multi-step activation for Snapcast streams:
                // 1. Switch amplifier to Snapcast input
                // 2. Select stream on Snapcast server
                (
                    self.snapcast_amplifier_source,
                    Some(stream_id.clone()),
                )
            }
            UnifiedSource::AmplifierInput { source, .. } => {
                // Single-step activation for amplifier inputs:
                // Just switch amplifier to specified input
                (*source, None)
            }
        }
    }
}

/// Result of source activation
#[derive(Debug)]
pub enum ActivationResult {
    /// Snapcast stream successfully activated
    SnapcastStreamActivated { stream_id: String },

    /// Amplifier input successfully activated
    AmplifierInputActivated { source: AmplifierSource },
}

/// Persistent state for unified source selection
#[derive(Debug, Serialize, Deserialize)]
pub struct UnifiedSourceState {
    /// Currently active unified source
    pub active_source: UnifiedSource,

    /// Timestamp of last update
    #[serde(with = "systemtime_serde")]
    pub last_updated: SystemTime,
}

impl UnifiedSourceState {
    /// Create new state with given source
    pub fn new(active_source: UnifiedSource) -> Self {
        Self {
            active_source,
            last_updated: SystemTime::now(),
        }
    }

    /// Save state to file
    pub fn save(&self) -> Result<()> {
        let state_file = dirs::config_dir()
            .context("Failed to get config directory")?
            .join("snapcast-controller/unified_source_state.json");

        // Create parent directory if needed
        if let Some(parent) = state_file.parent() {
            fs::create_dir_all(parent)?;
        }

        // Write JSON file
        let json = serde_json::to_string_pretty(&self)?;
        fs::write(state_file, json)?;

        Ok(())
    }

    /// Load state from file
    pub fn load() -> Result<Option<Self>> {
        let state_file = dirs::config_dir()
            .context("Failed to get config directory")?
            .join("snapcast-controller/unified_source_state.json");

        if !state_file.exists() {
            return Ok(None);
        }

        let json = fs::read_to_string(&state_file)?;
        let state: UnifiedSourceState = serde_json::from_str(&json)?;

        Ok(Some(state))
    }
}

/// Custom serde module for SystemTime
mod systemtime_serde {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::time::{SystemTime, UNIX_EPOCH};

    pub fn serialize<S>(time: &SystemTime, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let duration = time
            .duration_since(UNIX_EPOCH)
            .map_err(serde::ser::Error::custom)?;
        duration.as_secs().serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<SystemTime, D::Error>
    where
        D: Deserializer<'de>,
    {
        let secs = u64::deserialize(deserializer)?;
        Ok(UNIX_EPOCH + std::time::Duration::from_secs(secs))
    }
}
