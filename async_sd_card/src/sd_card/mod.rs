mod structs;

use bitfield::bitfield;
use bitflags::bitflags;
use crc::{CRC_7_MMC, Crc};
use defmt::info;
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

fn format_command_58() -> [u8; 6] {
    format_command(58, 0)
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
    spi_bus
        .write(&format_command_58())
        .await
        .map_err(SpiError::Bus)
        .map_err(Command58Error::Spi)?;
    // We're not allowed to talk to other SPI devices between sending the command and receiving a response
    loop {
        // Timer::after(Duration::from_millis(1000)).await;
        let mut buffer = [0xFF; 1];
        spi_bus
            .transfer_in_place(&mut buffer)
            .await
            .map_err(SpiError::Bus)
            .map_err(Command58Error::Spi)?;
        let r1 = R1::from_bits_retain(buffer[0]);
        if !r1.contains(R1::BIT_7) {
            if r1 != R1::IN_IDLE_STATE {
                cs.set_high()
                    .map_err(SpiError::Cs)
                    .map_err(Command58Error::Spi)?;
                spi_bus
                    .write(&[0xFF])
                    .await
                    .map_err(SpiError::Bus)
                    .map_err(Command58Error::Spi)?;
                return Err(Command58Error::R1Error(r1));
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
        .map_err(Command58Error::Spi)?;
    cs.set_high()
        .map_err(SpiError::Cs)
        .map_err(Command58Error::Spi)?;
    spi_bus
        .write(&[0xFF])
        .await
        .map_err(SpiError::Bus)
        .map_err(Command58Error::Spi)?;
    let ocr = Ocr::from_bits_retain(u32::from_be_bytes(buffer));
    Ok(ocr)
}
