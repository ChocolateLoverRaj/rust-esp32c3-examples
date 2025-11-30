use std::time::{Duration, Instant};

use esp_idf_svc::{
    hal::{
        gpio::{InterruptType, PinDriver, Pull},
        prelude::*,
        spi::{config::DriverConfig, SpiConfig, SpiDeviceDriver, SpiDriver},
    },
    sys::EspError,
};
use mfrc522::{comm::blocking::spi::SpiInterface, Mfrc522, RxGain, Uid};
use tokio::select;
use tokio::time::sleep;

fn main() {
    // It is necessary to call this function once. Otherwise, some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(main_async())
}

async fn main_async() {
    let peripherals = Peripherals::take().unwrap();

    // Reset the rc522 (optional, but good for when running new code without powering down the rc522)
    let mut reset_pin = PinDriver::output(peripherals.pins.gpio10).unwrap();
    reset_pin.set_low().unwrap();
    sleep(Duration::from_millis(10)).await;
    reset_pin.set_high().unwrap();
    sleep(Duration::from_millis(5)).await;

    let mut led = PinDriver::output(peripherals.pins.gpio8).unwrap();
    led.set_high().unwrap();

    let mut button = PinDriver::input(peripherals.pins.gpio9).unwrap();
    button.set_pull(Pull::Down).unwrap();
    button.set_interrupt_type(InterruptType::PosEdge).unwrap();
    button.enable_interrupt().unwrap();

    let driver = SpiDriver::new(
        peripherals.spi2,
        peripherals.pins.gpio4,       // SCK
        peripherals.pins.gpio6,       // PICO
        Some(peripherals.pins.gpio5), // POCI
        &DriverConfig::new(),
    )
    .unwrap();

    let spi_config = SpiConfig::new().baudrate(10.MHz().into());
    let spi = SpiDeviceDriver::new(driver, Some(peripherals.pins.gpio21), &spi_config).unwrap();
    let itf = SpiInterface::new(spi);

    let mut mfrc522 = Mfrc522::new(itf).init().unwrap();
    match mfrc522.version() {
        Ok(version) => log::info!("version {:x}", version),
        Err(_e) => log::error!("version error"),
    }

    let rx_gains = [
        RxGain::DB18,
        RxGain::DB23,
        RxGain::DB33,
        RxGain::DB38,
        RxGain::DB43,
        RxGain::DB48,
    ];
    let mut rx_gain_index = 0;
    {
        let rx_gain = rx_gains[rx_gain_index];
        mfrc522.set_antenna_gain(rx_gain).unwrap();
        log::info!("Initial rx gain: {rx_gain:?}");
    }

    let poll_interval = Duration::from_millis(20);
    // While a tag is present, its presence might not be continually read by the nfc reader
    // So we use a timeout before treating the tag as gone
    let tag_gone_timeout = poll_interval * 4;

    struct PreviousUid {
        uid: Uid,
        last_seen: Instant,
    }

    let mut previous_uid = None::<PreviousUid>;
    let mut uid_last_updated = Instant::now();
    loop {
        let now = Instant::now();
        let current_uid = if let Ok(atqa) = mfrc522.new_card_present() {
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

        let previous_present_uid = previous_uid.as_ref().and_then(|previous_uid| {
            if uid_last_updated - previous_uid.last_seen < tag_gone_timeout {
                Some(&previous_uid.uid)
            } else {
                None
            }
        });
        let current_present_uid =
            current_uid
                .as_ref()
                .or(previous_uid.as_ref().and_then(|previous_uid| {
                    if now - previous_uid.last_seen < tag_gone_timeout {
                        Some(&previous_uid.uid)
                    } else {
                        None
                    }
                }));

        // If the present uid changed, update the display (LED and logging)
        if previous_present_uid.map(Uid::as_bytes) != current_present_uid.map(Uid::as_bytes) {
            log::info!("UID: {:?}", current_present_uid.map(Uid::as_bytes));
            led.set_level(current_present_uid.is_none().into()).unwrap();
        }

        // Update the state
        if let Some(uid) = current_uid {
            previous_uid = Some(PreviousUid {
                uid,
                last_seen: now,
            });
        }
        uid_last_updated = now;

        enum S {
            Button(Result<(), EspError>),
            PollInterval,
        }
        let s = select! {
            result = button.wait_for_rising_edge() => S::Button(result),
            _ = sleep(poll_interval) => S::PollInterval
        };
        match s {
            S::Button(result) => {
                result.unwrap();
                rx_gain_index += 1;
                if rx_gain_index == rx_gains.len() {
                    rx_gain_index = 0;
                }
                {
                    let rx_gain = rx_gains[rx_gain_index];
                    mfrc522.set_antenna_gain(rx_gain).unwrap();
                    log::info!("Set rx gain to {rx_gain:?}");
                }
            }
            S::PollInterval => {}
        }
    }
}
