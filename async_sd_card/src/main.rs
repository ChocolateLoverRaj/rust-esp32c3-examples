#![no_std]
#![no_main]

mod sd_card;

use defmt::{error, info, warn};
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
    Command8Error, Command58Error, CsdV2, Ocr, R1, R7Byte1, R7Byte3, VoltageAccpted, command_0,
    command_8, command_9, command_55, command_58, command_a41, format_command, format_command_0,
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

    info!("sending CMD0");
    command_0(&mut spi_bus, &mut cs).await.unwrap();

    // Simulate talking to a different SPI device
    // SpiBus::write(&mut spi_bus, &[0xFF; 1000]).await.unwrap();

    info!("sending CMD8");
    // The check pattern can be anything we want
    let check_pattern = 0xE2;
    let mut result = command_8(&mut spi_bus, &mut cs, check_pattern).await;
    if let Err(Command8Error::VoltageNotSupported) = result {
        warn!("Voltage not supported. Proceeding anyways.");
        result = Ok(());
    }
    match result {
        Ok(()) => {
            info!("CMD8 Ok");

            // Simulate talking to a different SPI device
            SpiBus::write(&mut spi_bus, &[0xFF; 1000]).await.unwrap();

            info!("sending CMD58");
            let ocr = command_58(&mut spi_bus, &mut cs).await.unwrap();
            info!("OCR: 0b{:032b}", ocr.bits());
            assert!(ocr.supports_3_3v());

            // Simulate talking to a different SPI device
            SpiBus::write(&mut spi_bus, &[0xFF; 1000]).await.unwrap();

            loop {
                info!("sending CMD55");
                command_55(&mut spi_bus, &mut cs).await.unwrap();

                // Simulate talking to a different SPI device
                SpiBus::write(&mut spi_bus, &[0xFF; 1000]).await.unwrap();

                info!("sending ACMD41");
                let is_idle = command_a41(&mut spi_bus, &mut cs, true).await.unwrap();
                if is_idle {
                    info!("SD card is not ready yet");

                    // Simulate talking to a different SPI device
                    SpiBus::write(&mut spi_bus, &[0xFF; 1000]).await.unwrap();
                } else {
                    break;
                }
            }

            // Simulate talking to a different SPI device
            SpiBus::write(&mut spi_bus, &[0xFF; 1000]).await.unwrap();

            info!("sending CMD58");
            let ocr = command_58(&mut spi_bus, &mut cs).await.unwrap();
            if ocr.supports_sdhc_or_sdxc().expect("card not powered up") {
                info!("Card is in SDHC or SDXC mode");
            } else {
                info!("Card is standard capacity")
            }

            // Simulate talking to a different SPI device
            SpiBus::write(&mut spi_bus, &[0xFF; 1000]).await.unwrap();

            info!("CMD9");
            let csd = CsdV2(command_9(&mut spi_bus, &mut cs).await.unwrap());
            let capacity = csd.card_capacity_bytes();
            info!("Capacity: {}", capacity);
        }
        Err(Command8Error::IllegalCommand(r1)) => {
            error!("Illegal command: 0b{:08b}", r1.bits());
            todo!();
            info!("sending CMD58");
            let ocr = command_58(&mut spi_bus, &mut cs).await.unwrap();
            info!("OCR: 0b{:032b}", ocr.bits());
            assert!(ocr.supports_3_3v());

            loop {
                info!("sending CMD55");
                command_55(&mut spi_bus, &mut cs).await.unwrap();

                // Simulate talking to a different SPI device
                SpiBus::write(&mut spi_bus, &[0xFF; 1000]).await.unwrap();

                info!("sending ACMD41");
                let is_idle = command_a41(&mut spi_bus, &mut cs, false).await.unwrap();
                if is_idle {
                    info!("SD card is not ready yet");

                    // Simulate talking to a different SPI device
                    SpiBus::write(&mut spi_bus, &[0xFF; 1000]).await.unwrap();
                } else {
                    break;
                }
            }
            info!("SD card (version 1) is ready");
        }
        result => result.unwrap(),
    };
}
