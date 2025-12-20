#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embedded_hal_bus::spi::ExclusiveDevice;
use embedded_sdmmc::{SdCard, TimeSource, Timestamp, VolumeIdx, VolumeManager};
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

esp_bootloader_esp_idf::esp_app_desc!();

#[esp_rtos::main]
async fn main(spawner: Spawner) {
    let _ = spawner;

    let peripherals = esp_hal::init(Default::default());

    // Needed for esp_rtos
    let timg0 = TimerGroup::new(peripherals.TIMG0);
    let software_interrupt = SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    esp_rtos::start(timg0.timer0, software_interrupt.software_interrupt0);

    let sd_card = SdCard::new(
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
    );

    // Get the card size (this also triggers card initialisation because it's not been done yet)
    println!("Card size is {} bytes", sd_card.num_bytes().unwrap());
    // Now let's look for volumes (also known as partitions) on our block device.
    // To do this we need a Volume Manager. It will take ownership of the block device.
    let volume_mgr = VolumeManager::new(sd_card, {
        struct ZeroTimeSource;
        impl TimeSource for ZeroTimeSource {
            fn get_timestamp(&self) -> embedded_sdmmc::Timestamp {
                Timestamp {
                    year_since_1970: 0,
                    zero_indexed_month: 0,
                    zero_indexed_day: 0,
                    hours: 0,
                    minutes: 0,
                    seconds: 0,
                }
            }
        }
        ZeroTimeSource
    });
    // Try and access Volume 0 (i.e. the first partition).
    // The volume object holds information about the filesystem on that volume.
    let volume0 = volume_mgr.open_volume(VolumeIdx(0)).unwrap();
    println!("Volume 0: {:?}", volume0);
    // Open the root directory (mutably borrows from the volume).
    let root_dir = volume0.open_root_dir().unwrap();
    root_dir
        .iterate_dir(|dir_entry| println!("Dir entry: {dir_entry:#?}"))
        .unwrap();
}
