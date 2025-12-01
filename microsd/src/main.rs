use std::{thread::sleep, time::Duration};

use embedded_sdmmc::{
    sdcard::AcquireOpts, Mode, SdCard, TimeSource, Timestamp, VolumeIdx, VolumeManager,
};
use esp_idf_svc::hal::{
    delay::Delay,
    prelude::*,
    spi::{
        config::{DriverConfig, Duplex},
        SpiConfig, SpiDeviceDriver, SpiDriver,
    },
};
use humansize::{format_size, BINARY};

pub struct ZeroTimeSource;

impl TimeSource for ZeroTimeSource {
    fn get_timestamp(&self) -> Timestamp {
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

fn main() {
    // It is necessary to call this function once. Otherwise, some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    let peripherals = Peripherals::take().unwrap();

    sleep(Duration::from_secs(1));

    let driver = SpiDriver::new(
        peripherals.spi2,
        peripherals.pins.gpio4,       // SCK
        peripherals.pins.gpio6,       // PICO
        Some(peripherals.pins.gpio5), // POCI
        &DriverConfig::new(),
    )
    .unwrap();

    let spi_config = SpiConfig::new()
        .baudrate(24.MHz().into())
        .duplex(Duplex::Full);
    let spi = SpiDeviceDriver::new(driver, Some(peripherals.pins.gpio0), &spi_config).unwrap();

    // Build an SD Card interface out of an SPI device, a chip-select pin and the delay object
    let sdcard = SdCard::new_with_options(
        spi,
        Delay::default(),
        AcquireOpts {
            acquire_retries: 5,
            ..Default::default()
        },
    );
    // Get the card size (this also triggers card initialisation because it's not been done yet)
    println!(
        "Card size: {}",
        format_size(sdcard.num_bytes().unwrap(), BINARY)
    );
    // Now let's look for volumes (also known as partitions) on our block device.
    // To do this we need a Volume Manager. It will take ownership of the block device.
    let volume_mgr = VolumeManager::new(sdcard, ZeroTimeSource);
    // Try and access Volume 0 (i.e. the first partition).
    // The volume object holds information about the filesystem on that volume.
    let volume0 = volume_mgr.open_volume(VolumeIdx(0)).unwrap();
    // Open the root directory (mutably borrows from the volume).
    let root_dir = volume0.open_root_dir().unwrap();
    // Open a file called "MY_FILE.TXT" in the root directory
    // This mutably borrows the directory.
    let my_file = root_dir
        .open_file_in_dir("hello.txt", Mode::ReadOnly)
        .unwrap();
    // Print the contents of the file, assuming it's in ISO-8859-1 encoding
    while !my_file.is_eof() {
        let mut buffer = [0u8; 32];
        let num_read = my_file.read(&mut buffer).unwrap();
        for b in &buffer[0..num_read] {
            print!("{}", *b as char);
        }
    }
}
