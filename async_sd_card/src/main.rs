#![no_std]
#![no_main]

mod sd_card;

use defmt::info;
use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use embedded_hal_async::spi::SpiBus;
use embedded_hal_bus::spi::ExclusiveDevice;
use esp_backtrace as _;
use esp_hal::{
    delay::Delay,
    gpio::{Level, Output, OutputConfig},
    interrupt::software::SoftwareInterruptControl,
    spi::{
        self,
        master::{Config, Spi},
    },
    time::Rate,
    timer::timg::TimerGroup,
};
use esp_println::{self as _, println};

use crate::sd_card::{
    R1, R7Byte1, R7Byte3, VoltageAccpted, command_0, command_8, format_command, format_command_0,
    format_command_8,
};

esp_bootloader_esp_idf::esp_app_desc!();

#[esp_rtos::main]
async fn main(spawner: Spawner) {
    let _ = spawner;

    let peripherals = esp_hal::init(Default::default());

    // Needed for esp_rtos
    let timg0 = TimerGroup::new(peripherals.TIMG0);
    let software_interrupt = SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    esp_rtos::start(timg0.timer0, software_interrupt.software_interrupt0);

    let mut spi_bus = Spi::new(
        peripherals.SPI2,
        Config::default().with_frequency(Rate::from_khz(400)),
    )
    .unwrap()
    .with_sck(peripherals.GPIO7)
    .with_mosi(peripherals.GPIO6)
    .with_miso(peripherals.GPIO5)
    .into_async();

    let mut cs = Output::new(peripherals.GPIO0, Level::High, OutputConfig::default());

    // The spec says to wait 1ms from when the SD card gets power
    // Realistically it has already been 1ms but just to be sure we can wait 1ms
    info!("waiting for 1ms");
    Timer::after(Duration::from_millis(1)).await;

    info!("waiting at least 74 clock cycles");
    // Send 0xFF for at least 74 clock cycles according to the spec
    // So 9 bytes
    SpiBus::write(&mut spi_bus, &[0xFF; 9]).await.unwrap();

    // Send CMD0
    info!("sending CMD0");
    command_0(&mut spi_bus, &mut cs).await.unwrap();

    // Simulate talking to a different SPI device
    SpiBus::write(&mut spi_bus, &[0xFF; 1000]).await.unwrap();

    // Send CMD8
    info!("sending CMD8");
    // The check pattern can be anything we want
    let check_pattern = 0xE2;
    command_8(&mut spi_bus, &mut cs, check_pattern)
        .await
        .unwrap();
    info!("CMD8 Ok");
}
