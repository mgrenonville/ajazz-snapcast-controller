/// RC5 IR protocol codes for amplifier control
///
/// These codes are sent via MQTT to a Tasmota IR Blaster device
/// Format: RC5 protocol, 12 bits

/// IR codes for amplifier input source selection
pub mod sources {
    /// Phono input (turntable)
    pub const PHONO: u16 = 0xC01;

    /// CD player input
    pub const CD: u16 = 0xC02;

    /// Spotify/streaming input
    pub const SPOTIFY: u16 = 0xC03;

    /// Source 4 input
    pub const SOURCE_4: u16 = 0xC04;

    /// Source 5 input
    pub const SOURCE_5: u16 = 0xC05;
}

/// IR codes for amplifier volume control
pub mod volume {
    /// Volume up command
    pub const UP: u16 = 0xC10;

    /// Volume down command
    pub const DOWN: u16 = 0xC11;
}

/// RC5 protocol constants
pub mod protocol {
    /// Protocol name for Tasmota IR Blaster
    pub const NAME: &str = "RC5";

    /// Number of bits in RC5 protocol
    pub const BITS: u8 = 12;

    /// Repeat count (0 = send once)
    pub const REPEAT: u8 = 0;
}
