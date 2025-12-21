mod structs;

use crc::{CRC_7_MMC, CRC_16_XMODEM, Crc};
use defmt::warn;
use embassy_time::Timer;
use embedded_hal::digital::OutputPin;
use embedded_hal_async::spi::SpiBus;
pub use structs::*;

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

pub fn format_command_0() -> [u8; 6] {
    format_command(0, 0)
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

/// Does not modify CS
pub async fn card_command<S: SpiBus>(spi_bus: &mut S, command: &[u8; 6]) -> Result<R1, S::Error> {
    spi_bus.write(command).await?;
    let r1 = loop {
        let mut buffer = [0xFF; 1];
        spi_bus.transfer_in_place(&mut buffer).await?;
        let r1 = R1::from_bits_retain(buffer[0]);
        if !r1.contains(R1::BIT_7) {
            break r1;
        } else {
            Timer::after_micros(10).await;
            // TODO: Timeout
        }
    };
    Ok(r1)
}

pub async fn command_0<Bus: SpiBus, Cs: OutputPin>(
    spi_bus: &mut Bus,
    cs: &mut Cs,
) -> Result<(), Command0Error<Bus::Error, Cs::Error>> {
    cs.set_low()
        .map_err(SpiError::Cs)
        .map_err(Command0Error::Spi)?;
    let result = {
        let r1 = card_command(spi_bus, &format_command_0())
            .await
            .map_err(|e| Command0Error::Spi(SpiError::Bus(e)))?;
        if r1 == R1::IN_IDLE_STATE {
            Ok(())
        } else {
            Err(Command0Error::R1Error(r1))
        }
    };
    cs.set_high()
        .map_err(SpiError::Cs)
        .map_err(Command0Error::Spi)?;
    spi_bus
        .write(&[0xFF])
        .await
        .map_err(|e| Command0Error::Spi(SpiError::Bus(e)))?;
    result
}

#[derive(Debug)]
pub enum Command8Error<BusError, CsError> {
    Spi(SpiError<BusError, CsError>),
    /// Cards that don't support version 2 will send this
    R1Error(R1),
    CheckPatternMismatch(u8),
    /// The SD Card does not support 3.3V
    VoltageNotSupported,
}

/// This function assumes that you are providing 2.6V-3.6V to the SD card.
pub async fn command_8<Bus: SpiBus, Cs: OutputPin>(
    spi_bus: &mut Bus,
    cs: &mut Cs,
    check_pattern: u8,
) -> Result<(), Command8Error<Bus::Error, Cs::Error>> {
    cs.set_low()
        .map_err(SpiError::Cs)
        .map_err(Command8Error::Spi)?;
    let r1 = card_command(
        spi_bus,
        &format_command_8(false, false, VoltageAccpted::_2_7V_3_6V, check_pattern),
    )
    .await
    .map_err(|e| Command8Error::Spi(SpiError::Bus(e)))?;
    let result = {
        if r1 == R1::IN_IDLE_STATE {
            let mut buffer = [0xFF; 4];
            spi_bus
                .transfer_in_place(&mut buffer)
                .await
                .map_err(SpiError::Bus)
                .map_err(Command8Error::Spi)?;

            let response_check_pattern = buffer[3];
            if response_check_pattern == check_pattern {
                let byte_3 = R7Byte3(buffer[2]);
                if byte_3
                    .get_voltage_accepted()
                    .contains(VoltageAccpted::_2_7V_3_6V)
                {
                    Ok(())
                } else {
                    Err(Command8Error::VoltageNotSupported)
                }
            } else {
                Err(Command8Error::CheckPatternMismatch(response_check_pattern))
            }
        } else {
            Err(Command8Error::R1Error(r1))
        }
    };
    cs.set_high()
        .map_err(SpiError::Cs)
        .map_err(Command8Error::Spi)?;
    spi_bus
        .write(&[0xFF])
        .await
        .map_err(SpiError::Bus)
        .map_err(Command8Error::Spi)?;

    result
}

#[derive(Debug)]
pub enum Command58Error<BusError, CsError> {
    Spi(SpiError<BusError, CsError>),
    R1Error(R1),
}

pub async fn command_58<Bus: SpiBus, Cs: OutputPin>(
    spi_bus: &mut Bus,
    cs: &mut Cs,
) -> Result<Ocr, Command58Error<Bus::Error, Cs::Error>> {
    cs.set_low()
        .map_err(SpiError::Cs)
        .map_err(Command58Error::Spi)?;
    let r1 = card_command(spi_bus, &format_command(58, 0))
        .await
        .map_err(|e| Command58Error::Spi(SpiError::Bus(e)))?;
    // We're not allowed to talk to other SPI devices between sending the command and receiving a response
    let result = {
        // CMD58 can be called before or after ACMD41
        // The spec suggests calling it before ACMD41 to check that the voltage is compatible
        // So it's ok if R1 is idle or not idle
        if r1 == R1::IN_IDLE_STATE || r1.is_empty() {
            let mut buffer = [0xFF; 4];
            spi_bus
                .transfer_in_place(&mut buffer)
                .await
                .map_err(SpiError::Bus)
                .map_err(Command58Error::Spi)?;
            let ocr = Ocr::from_bits_retain(u32::from_be_bytes(buffer));
            Ok(ocr)
        } else {
            Err(Command58Error::R1Error(r1))
        }
    };
    cs.set_high()
        .map_err(SpiError::Cs)
        .map_err(Command58Error::Spi)?;
    spi_bus
        .write(&[0xFF])
        .await
        .map_err(SpiError::Bus)
        .map_err(Command58Error::Spi)?;
    result
}

#[derive(Debug)]
pub enum Command55Error<BusError, CsError> {
    Spi(SpiError<BusError, CsError>),
    R1Error(R1),
}

pub async fn command_55<Bus: SpiBus, Cs: OutputPin>(
    spi_bus: &mut Bus,
    cs: &mut Cs,
) -> Result<(), Command55Error<Bus::Error, Cs::Error>> {
    cs.set_low()
        .map_err(SpiError::Cs)
        .map_err(Command55Error::Spi)?;
    spi_bus
        .write(&format_command(55, 0))
        .await
        .map_err(SpiError::Bus)
        .map_err(Command55Error::Spi)?;
    // We're not allowed to talk to other SPI devices between sending the command and receiving a response
    let result = loop {
        // Timer::after(Duration::from_millis(1000)).await;
        let mut buffer = [0xFF; 1];
        spi_bus
            .transfer_in_place(&mut buffer)
            .await
            .map_err(SpiError::Bus)
            .map_err(Command55Error::Spi)?;
        let r1 = R1::from_bits_retain(buffer[0]);
        if !r1.contains(R1::BIT_7) {
            break if r1 == R1::IN_IDLE_STATE {
                Ok(())
            } else {
                Err(Command55Error::R1Error(r1))
            };
        } else {
            // TODO: Timeout
        }
    };
    cs.set_high()
        .map_err(SpiError::Cs)
        .map_err(Command55Error::Spi)?;
    spi_bus
        .write(&[0xFF])
        .await
        .map_err(SpiError::Bus)
        .map_err(Command55Error::Spi)?;
    result
}

#[derive(Debug)]
pub enum CommandA41Error<BusError, CsError> {
    Spi(SpiError<BusError, CsError>),
    R1Error(R1),
}

/// Returns if the SD card is idle
pub async fn command_a41<Bus: SpiBus, Cs: OutputPin>(
    spi_bus: &mut Bus,
    cs: &mut Cs,
    host_supports_hcs: bool,
) -> Result<bool, CommandA41Error<Bus::Error, Cs::Error>> {
    cs.set_low()
        .map_err(SpiError::Cs)
        .map_err(CommandA41Error::Spi)?;
    let r1 = card_command(
        spi_bus,
        &format_command(41, {
            let mut argument = CommandA41Argument::default();
            argument.set(CommandA41Argument::HCS, host_supports_hcs);
            argument.bits()
        }),
    )
    .await
    .map_err(|e| CommandA41Error::Spi(SpiError::Bus(e)))?;
    // We're not allowed to talk to other SPI devices between sending the command and receiving a response
    let result = {
        if r1 == R1::IN_IDLE_STATE || r1.is_empty() {
            Ok(r1.contains(R1::IN_IDLE_STATE))
        } else {
            Err(CommandA41Error::R1Error(r1))
        }
    };
    cs.set_high()
        .map_err(SpiError::Cs)
        .map_err(CommandA41Error::Spi)?;
    spi_bus
        .write(&[0xFF])
        .await
        .map_err(SpiError::Bus)
        .map_err(CommandA41Error::Spi)?;
    result
}

#[derive(Debug)]
pub enum Command9Error<BusError, CsError> {
    Spi(SpiError<BusError, CsError>),
    R1Error(R1),
    InvalidChecksum,
}
pub async fn command_9<Bus: SpiBus, Cs: OutputPin>(
    spi_bus: &mut Bus,
    cs: &mut Cs,
) -> Result<u128, Command9Error<Bus::Error, Cs::Error>> {
    cs.set_low()
        .map_err(SpiError::Cs)
        .map_err(Command9Error::Spi)?;
    let r1 = card_command(spi_bus, &format_command(9, 0))
        .await
        .map_err(|e| Command9Error::Spi(SpiError::Bus(e)))?;
    let result = if r1.is_empty() {
        // TODO: Are we allowed to talk to other SPI devices during this time?
        loop {
            let mut buffer = [0xFF; 1];
            spi_bus
                .transfer_in_place(&mut buffer)
                .await
                .map_err(SpiError::Bus)
                .map_err(Command9Error::Spi)?;
            let byte = buffer[0];
            if byte != 0xFF {
                break;
            } else {
                // TODO: Timeout
            }
        }
        let mut buffer = [0xFF; 18];
        spi_bus
            .transfer_in_place(&mut buffer)
            .await
            .map_err(SpiError::Bus)
            .map_err(Command9Error::Spi)?;
        let (csd, crc) = buffer.split_at(16);
        let csd = <&[u8; 16]>::try_from(csd).unwrap();
        let crc = u16::from_be_bytes(*<&[u8; 2]>::try_from(crc).unwrap());
        if crc == Crc::<u16>::new(&CRC_16_XMODEM).checksum(csd) {
            Ok(u128::from_be_bytes(*csd))
        } else {
            Err(Command9Error::InvalidChecksum)
        }
    } else {
        return Err(Command9Error::R1Error(r1));
    };
    cs.set_high()
        .map_err(SpiError::Cs)
        .map_err(Command9Error::Spi)?;
    spi_bus
        .write(&[0xFF])
        .await
        .map_err(SpiError::Bus)
        .map_err(Command9Error::Spi)?;
    result
}
