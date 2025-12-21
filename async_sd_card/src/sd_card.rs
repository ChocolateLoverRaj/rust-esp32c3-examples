use bitfield::bitfield;
use bitflags::bitflags;
use crc::{CRC_7_MMC, Crc};
use defmt::info;
use embedded_hal::digital::OutputPin;
use embedded_hal_async::spi::SpiBus;

pub fn format_command(command_index: u8, argument: u32) -> [u8; 6] {
    let mut command: [u8; 6] = Default::default();
    command[0] = {
        let mut byte = CommandByte0(Default::default());
        byte.set_start_bit(false);
        byte.set_transmission_bit(true);
        byte.set_command_index(command_index);
        byte.0
    };
    command[1..5].copy_from_slice(&argument.to_be_bytes());
    command[5] = {
        let mut byte = CommandByte5(Default::default());
        byte.set_crc7(Crc::<u8>::new(&CRC_7_MMC).checksum(&command[..5]));
        byte.set_end_bit(true);
        byte.0
    };

    command
}

bitfield! {
    pub struct CommandByte0(u8);

    bool; pub get_start_bit, set_start_bit: 7;
    bool; pub get_transmission_bit, set_transmission_bit: 6;
    u8; pub get_command_index, set_command_index: 5, 0;
}

bitfield! {
    pub struct CommandByte5(u8);

    u8; pub get_crc7, set_crc7: 7, 1;
    bool; pub get_end_bit, set_end_bit: 0;
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct R1: u8 {
        const BIT_7 = 1 << 7;
        const PARAMETER_ERROR = 1 << 6;
        const ADDRESS_ERROR = 1 << 5;
        const ERASE_SEQUENCE_ERROR = 1 << 4;
        const COM_CRC_ERROR = 1 << 3;
        const ILLEGAL_COMMAND = 1 << 2;
        const ERASE_RESET = 1 << 1;
        const IN_IDLE_STATE = 1 << 0;
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct R2Byte2: u8 {
        const OUT_OF_RANGE_OR_CSD_OVERWRITE = 1 << 7;
        const ERASE_PARAM = 1 << 6;
        const WP_VIOLATION = 1 << 5;
        const CARD_ECC_FAILED = 1 << 4;
        const CC_ERROR = 1 << 3;
        const ERROR = 1 << 2;
        const WP_ERASE_SKIP_OR_LOCK_UNLOCK_CMD_FAILED = 1 << 1;
        const CARD_IS_LOCKED = 1 << 0;
    }
}

bitfield! {
    pub struct R7Byte1(u8);

    u8; pub get_command_version, set_command_version: 7, 4;
}

bitfield! {
    pub struct R7Byte3(u8);

    u8; _get_voltage_accepted, _set_volage_accepted: 3, 0;
}

impl R7Byte3 {
    pub fn get_voltage_accepted(&self) -> VoltageAccpted {
        VoltageAccpted::from_bits_retain(self._get_voltage_accepted())
    }
}

pub fn format_command_0() -> [u8; 6] {
    format_command(0, 0)
}

bitfield! {
    pub struct Command8Argument(u32);

    bool; pub get_pcie_1_2v_support, set_pcie1_2v_support: 13;
    bool; pub get_pcie_availability, set_pcie_availability: 12;
    u8; pub get_voltage_accpted, set_voltage_accepted: 11, 8;
    u8; pub get_check_pattern, set_check_pattern: 7, 0;
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct VoltageAccpted: u8 {
        const _2_7V_3_6V = 1 << 1;
    }
}

pub fn format_command_8(
    ask_pcie_1_2v_support: bool,
    ask_pcie_availability: bool,
    ask_voltage_accepted: VoltageAccpted,
    check_pattern: u8,
) -> [u8; 6] {
    format_command(8, {
        let mut argument = Command8Argument(Default::default());
        argument.set_pcie1_2v_support(ask_pcie_1_2v_support);
        argument.set_pcie_availability(ask_pcie_availability);
        argument.set_voltage_accepted(ask_voltage_accepted.bits());
        argument.set_check_pattern(check_pattern);
        argument.0
    })
}

#[derive(Debug)]
pub enum SpiError<BusError, CsError> {
    Bus(BusError),
    Cs(CsError),
}

#[derive(Debug)]
pub enum Command0Error<BusError, CsError> {
    Spi(SpiError<BusError, CsError>),
    R1Error(R1),
}

pub async fn command_0<Bus: SpiBus, Cs: OutputPin>(
    spi_bus: &mut Bus,
    cs: &mut Cs,
) -> Result<(), Command0Error<Bus::Error, Cs::Error>> {
    cs.set_low()
        .map_err(SpiError::Cs)
        .map_err(Command0Error::Spi)?;
    spi_bus
        .write(&format_command_0())
        .await
        .map_err(SpiError::Bus)
        .map_err(Command0Error::Spi)?;
    // We're not allowed to talk to other SPI devices between sending the command and receiving a response
    let r1 = loop {
        // Timer::after(Duration::from_millis(1000)).await;
        let mut buffer = [0xFF; 1];
        spi_bus
            .transfer_in_place(&mut buffer)
            .await
            .map_err(SpiError::Bus)
            .map_err(Command0Error::Spi)?;
        let r1 = R1::from_bits_retain(buffer[0]);
        if !r1.contains(R1::BIT_7) {
            break r1;
        } else {
            // TODO: Timeout
        }
    };
    cs.set_high()
        .map_err(SpiError::Cs)
        .map_err(Command0Error::Spi)?;
    spi_bus
        .write(&[0xFF])
        .await
        .map_err(SpiError::Bus)
        .map_err(Command0Error::Spi)?;
    if r1 == R1::IN_IDLE_STATE {
        Ok(())
    } else {
        Err(Command0Error::R1Error(r1))
    }
}

#[derive(Debug)]
pub enum Command8Error<BusError, CsError> {
    Spi(SpiError<BusError, CsError>),
    /// Cards that don't support version 2 will send this
    IllegalCommand,
    CheckPatternMismatch(u8),
    /// The SD Card does not support 3.3V
    VoltageNotSupported,
}

pub async fn command_8<Bus: SpiBus, Cs: OutputPin>(
    spi_bus: &mut Bus,
    cs: &mut Cs,
    check_pattern: u8,
) -> Result<(), Command8Error<Bus::Error, Cs::Error>> {
    cs.set_low()
        .map_err(SpiError::Cs)
        .map_err(Command8Error::Spi)?;
    spi_bus
        .write(&format_command_8(
            false,
            false,
            VoltageAccpted::_2_7V_3_6V,
            check_pattern,
        ))
        .await
        .map_err(SpiError::Bus)
        .map_err(Command8Error::Spi)?;
    // We're not allowed to talk to other SPI devices between sending the command and receiving a response
    loop {
        // Timer::after(Duration::from_millis(1000)).await;
        let mut buffer = [0xFF; 1];
        spi_bus
            .transfer_in_place(&mut buffer)
            .await
            .map_err(SpiError::Bus)
            .map_err(Command8Error::Spi)?;
        let r1 = R1::from_bits_retain(buffer[0]);
        if !r1.contains(R1::BIT_7) {
            if r1.contains(R1::ILLEGAL_COMMAND) {
                cs.set_high()
                    .map_err(SpiError::Cs)
                    .map_err(Command8Error::Spi)?;
                spi_bus
                    .write(&[0xFF])
                    .await
                    .map_err(SpiError::Bus)
                    .map_err(Command8Error::Spi)?;
                return Err(Command8Error::IllegalCommand);
            }
            break;
        } else {
            // TODO: Timeout
        }
    }
    let mut buffer = [0xFF; 4];
    spi_bus
        .transfer_in_place(&mut buffer)
        .await
        .map_err(SpiError::Bus)
        .map_err(Command8Error::Spi)?;
    cs.set_high()
        .map_err(SpiError::Cs)
        .map_err(Command8Error::Spi)?;
    spi_bus
        .write(&[0xFF])
        .await
        .map_err(SpiError::Bus)
        .map_err(Command8Error::Spi)?;

    let response_check_pattern = buffer[3];
    if response_check_pattern != check_pattern {
        return Err(Command8Error::CheckPatternMismatch(response_check_pattern));
    }

    let byte_1 = R7Byte1(buffer[0]);
    info!("CMD8 version: {:02X}", byte_1.get_command_version());

    let byte_3 = R7Byte3(buffer[2]);
    if !byte_3
        .get_voltage_accepted()
        .contains(VoltageAccpted::_2_7V_3_6V)
    {
        return Err(Command8Error::VoltageNotSupported);
    }
    Ok(())
}
