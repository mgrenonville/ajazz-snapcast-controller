/// IR command builders for amplifier control

use super::ir_codes;
use super::types::{AmplifierSource, IrCommand};

/// Build IR command for source selection
///
/// # Arguments
/// * `source` - The amplifier source to select
///
/// # Returns
/// IrCommand ready to be sent via MQTT to the IR Blaster
pub fn build_source_command(source: AmplifierSource) -> IrCommand {
    let data_code = source.ir_code();
    IrCommand::new(data_code)
}

/// Build IR command for volume up
///
/// # Returns
/// IrCommand ready to be sent via MQTT to the IR Blaster
pub fn build_volume_up_command() -> IrCommand {
    IrCommand::new(ir_codes::volume::UP)
}

/// Build IR command for volume down
///
/// # Returns
/// IrCommand ready to be sent via MQTT to the IR Blaster
pub fn build_volume_down_command() -> IrCommand {
    IrCommand::new(ir_codes::volume::DOWN)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_source_command_phono() {
        let cmd = build_source_command(AmplifierSource::Phono);
        assert_eq!(cmd.protocol, "RC5");
        assert_eq!(cmd.bits, 12);
        assert_eq!(cmd.data, "0xC01");
        assert_eq!(cmd.repeat, 0);
    }

    #[test]
    fn test_source_command_cd() {
        let cmd = build_source_command(AmplifierSource::CD);
        assert_eq!(cmd.data, "0xC02");
    }

    #[test]
    fn test_volume_up_command() {
        let cmd = build_volume_up_command();
        assert_eq!(cmd.data, "0xC10");
    }

    #[test]
    fn test_volume_down_command() {
        let cmd = build_volume_down_command();
        assert_eq!(cmd.data, "0xC11");
    }
}
