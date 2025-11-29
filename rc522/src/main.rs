use std::thread::sleep;
use std::time::Duration;

use esp_idf_svc::hal::{
    prelude::*,
    spi::{config::DriverConfig, SpiConfig, SpiDeviceDriver, SpiDriver},
    task::block_on,
};
use mfrc522::{comm::blocking::spi::SpiInterface, Mfrc522, Uid};

fn main() {
    block_on(main_async());
}

async fn main_async() {
    // It is necessary to call this function once. Otherwise, some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    let peripherals = Peripherals::take().unwrap();

    let driver = SpiDriver::new(
        peripherals.spi2,
        peripherals.pins.gpio4,       // SCK
        peripherals.pins.gpio6,       // PICO
        Some(peripherals.pins.gpio5), // POCI
        &DriverConfig::new(),
    )
    .unwrap();

    let spi_config = SpiConfig::new().baudrate(10.MHz().into());
    let spi = SpiDeviceDriver::new(driver, Some(peripherals.pins.gpio10), &spi_config).unwrap();
    let itf = SpiInterface::new(spi);

    let mut mfrc522 = Mfrc522::new(itf).init().unwrap();
    match mfrc522.version() {
        Ok(version) => log::info!("version {:x}", version),
        Err(_e) => log::error!("version error"),
    }
    let mut last_uid = None::<Uid>;
    loop {
        let new_last_uid = if let Ok(atqa) = mfrc522.new_card_present() {
            match mfrc522.select(&atqa) {
                Ok(uid) => Some(uid),
                Err(e) => {
                    log::warn!("Select error: {e}");
                    None
                }
            }
        } else {
            None
        };
        let last_uid_changed = match (&last_uid, &new_last_uid) {
            (Some(last_uid), Some(new_last_uid)) => last_uid.as_bytes() == new_last_uid.as_bytes(),
            (None, None) => false,
            _ => true,
        };
        if last_uid_changed {
            last_uid = new_last_uid;
            log::info!("UID: {:?}", last_uid.as_ref().map(Uid::as_bytes));
        }
        sleep(Duration::from_millis(100));
    }
}
