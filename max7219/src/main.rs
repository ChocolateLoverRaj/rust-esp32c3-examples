use std::{
    thread::{self, sleep},
    time::Duration,
};

use esp_idf_hal::{
    gpio::{DriveStrength, Gpio0, InterruptType, PinDriver, Pull},
    peripherals::Peripherals,
    spi::{
        config::{Duplex, MODE_0},
        Dma, SpiConfig, SpiDeviceDriver, SpiDriver,
    },
    task::block_on,
    units::FromValueType,
};
use esp_idf_svc::nvs::{EspNvs, EspNvsPartition, NvsDefault};
use esp_idf_sys as _;
use max7219::MAX7219;

fn main() -> anyhow::Result<()> {
    // It is necessary to call this function once,
    // or else some patches to the runtime implemented by esp-idf-sys might not link properly.
    esp_idf_sys::link_patches();

    let peripherals = Peripherals::take()?;
    let driver = SpiDriver::new(
        peripherals.spi2,
        peripherals.pins.gpio4,
        peripherals.pins.gpio6, // PICO
        None::<Gpio0>,          // POCI
        &Default::default(),
    )?;
    let mut spi_config = SpiConfig::default();
    spi_config.data_mode = MODE_0;
    spi_config.baudrate = 10_u32.MHz().into();
    let spi = SpiDeviceDriver::new(driver, Some(peripherals.pins.gpio21), &spi_config).unwrap();
    let mut display = MAX7219::from_spi(1, spi).unwrap();
    display.power_on().unwrap();
    display.set_intensity(0, 0x1).unwrap();
    display
        .write_raw(
            0,
            &[
                0b10101010, // Row 0
                0b01010101, // Row 1
                0b10101010, // Row 2
                0b01010101, // Row 3
                0b10101010, // Row 4
                0b01010101, // Row 5
                0b10101010, // Row 6
                0b01010101, // Row 7
            ],
        )
        .unwrap();
    Ok(())
}
