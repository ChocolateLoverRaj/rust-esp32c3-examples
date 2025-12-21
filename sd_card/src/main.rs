#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use embedded_hal_bus::spi::ExclusiveDevice;
use embedded_sdmmc::{SdCard, SdCardError, sdcard::AcquireOpts};
use esp_backtrace as _;
use esp_hal::{
    delay::Delay,
    gpio::{Level, Output, OutputConfig},
    interrupt::software::SoftwareInterruptControl,
    spi::master::{Config, Spi},
    time::Rate,
    timer::timg::TimerGroup,
};
use esp_println::{self as _, println};
use humansize::{BINARY, SizeFormatter};

esp_bootloader_esp_idf::esp_app_desc!();

#[esp_rtos::main]
async fn main(spawner: Spawner) {
    let _ = spawner;

    let peripherals = esp_hal::init(Default::default());

    // Needed for esp_rtos
    let timg0 = TimerGroup::new(peripherals.TIMG0);
    let software_interrupt = SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    esp_rtos::start(timg0.timer0, software_interrupt.software_interrupt0);

    let sd_card = SdCard::new_with_options(
        ExclusiveDevice::new(
            Spi::new(
                peripherals.SPI2,
                Config::default().with_frequency(Rate::from_mhz(2)),
            )
            .unwrap()
            .with_sck(peripherals.GPIO7)
            .with_mosi(peripherals.GPIO6)
            .with_miso(peripherals.GPIO5),
            Output::new(peripherals.GPIO0, Level::High, OutputConfig::default()),
            Delay::new(),
        )
        .unwrap(),
        Delay::new(),
        AcquireOpts {
            // acquire_retries: 1,
            use_crc: false,
            ..Default::default()
        },
    );

    let mut prev_card_present = None;
    loop {
        let card_num_bytes = match sd_card.num_bytes() {
            Ok(num_bytes) => Ok(Some(num_bytes)),
            Err(e) => match e {
                SdCardError::CardNotFound => Ok(None),
                e => Err(e),
            },
        };
        match card_num_bytes {
            Ok(num_bytes) => {
                let card_present = Some(num_bytes.is_some());
                if card_present != prev_card_present {
                    match num_bytes {
                        Some(num_bytes) => {
                            let size = SizeFormatter::new(num_bytes, BINARY);
                            println!("Card detected with size {size}");
                            sd_card.mark_card_uninit();
                        }
                        None => {
                            println!("No card prsent");
                        }
                    }
                    prev_card_present = card_present;
                }
            }
            Err(e) => println!("Err: {e:#?}"),
        }
        Timer::after(Duration::from_secs(200000)).await;
    }
}
