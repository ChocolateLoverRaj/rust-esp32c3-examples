use std::fmt::Debug;

use esp_idf_hal::gpio::{Input, InputPin, InterruptType, OutputPin, PinDriver, Pull};
use esp_idf_sys::EspError;
use hal::blocking::i2c::{Write, WriteRead};
use vl53l0x::VL53L0x;

pub struct AsyncVL53L0x<'d, I2c: hal::blocking::i2c::WriteRead, Gpio1: InputPin> {
    pub vl53l0x: VL53L0x<I2c>,
    gpio1: PinDriver<'d, Gpio1, Input>,
}

#[derive(Debug)]
pub enum NewWithGpio1Error<I2cError> {
    NewVL53L0xError(vl53l0x::Error<I2cError>),
    PinDriverError(EspError),
    SetPullError(EspError),
    SetInterruptTypeError(EspError),
    EnableInterruptError(EspError),
}

#[derive(Debug)]
pub enum ReadRangeMmAsyncError<I2cError> {
    WaitForFallingEdgeError(EspError),
    ReadRangeError(nb::Error<vl53l0x::Error<I2cError>>),
}

impl<
        'd,
        I2c: WriteRead<Error = I2cError> + Write<Error = I2cError>,
        I2cError,
        Gpio1: InputPin + OutputPin,
    > AsyncVL53L0x<'d, I2c, Gpio1>
{
    pub fn new_with_gpio1(i2c: I2c, gpio1: Gpio1) -> Result<Self, NewWithGpio1Error<I2cError>> {
        let vl53l0x = VL53L0x::new(i2c).map_err(|e| NewWithGpio1Error::NewVL53L0xError(e))?;
        let mut gpio1 =
            PinDriver::input(gpio1).map_err(|e| NewWithGpio1Error::PinDriverError(e))?;
        gpio1
            .set_pull(Pull::Up)
            .map_err(|e| NewWithGpio1Error::SetPullError(e))?;
        gpio1
            .set_interrupt_type(InterruptType::NegEdge)
            .map_err(|e| NewWithGpio1Error::SetInterruptTypeError(e))?;
        gpio1
            .enable_interrupt()
            .map_err(|e| NewWithGpio1Error::EnableInterruptError(e))?;
        Ok(Self { vl53l0x, gpio1 })
    }

    pub async fn read_range_mm_async(&mut self) -> Result<u16, ReadRangeMmAsyncError<I2cError>> {
        self.gpio1
            .wait_for_falling_edge()
            .await
            .map_err(|e| ReadRangeMmAsyncError::WaitForFallingEdgeError(e))?;
        Ok(self
            .vl53l0x
            .read_range_mm()
            .map_err(|e| ReadRangeMmAsyncError::ReadRangeError(e))?)
    }
}
