#![no_std]
#![no_main]

use defmt::{error, info, warn};
use embassy_executor::Spawner;
use embassy_time::{Delay, Duration, Instant, Timer};
use embedded_hal_bus::spi::ExclusiveDevice;
use esp_backtrace as _;
use esp_hal::{
    gpio::{Input, InputConfig, Level, Output, OutputConfig, Pull},
    interrupt::software::SoftwareInterruptControl,
    spi::{
        Mode,
        master::{Config, Spi},
    },
    time::Rate,
    timer::timg::TimerGroup,
};
use esp_println as _;
use futures::{
    future::{Either, select},
    pin_mut,
};
use mfrc522::{Mfrc522, RxGain, Uid, comm::blocking::spi::SpiInterface};

esp_bootloader_esp_idf::esp_app_desc!();

#[esp_rtos::main]
async fn main(spawner: Spawner) {
    let _ = spawner;

    let peripherals = esp_hal::init(Default::default());

    // Needed for esp_rtos
    let timg0 = TimerGroup::new(peripherals.TIMG0);
    let software_interrupt = SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    esp_rtos::start(timg0.timer0, software_interrupt.software_interrupt0);

    // Reset the rc522 (optional, but good for when running new code without powering down the rc522)
    let mut reset_pin = Output::new(peripherals.GPIO10, Level::Low, OutputConfig::default());
    Timer::after(Duration::from_millis(10)).await;
    reset_pin.set_high();
    Timer::after(Duration::from_millis(5)).await;

    let mut led = Output::new(peripherals.GPIO8, Level::High, OutputConfig::default());

    let mut button = Input::new(
        peripherals.GPIO9,
        InputConfig::default().with_pull(Pull::Down),
    );

    let spi = Spi::new(
        peripherals.SPI2,
        Config::default()
            .with_frequency(Rate::from_mhz(10))
            .with_mode(Mode::_0),
    )
    .unwrap()
    .with_sck(peripherals.GPIO4)
    .with_mosi(peripherals.GPIO6)
    .with_miso(peripherals.GPIO5);

    let mut mfrc522 = Mfrc522::new(SpiInterface::new(
        ExclusiveDevice::new(
            spi,
            Output::new(peripherals.GPIO21, Level::High, OutputConfig::default()),
            Delay,
        )
        .unwrap(),
    ))
    .init()
    .unwrap();
    match mfrc522.version() {
        Ok(version) => info!("version 0x{:x}", version),
        Err(_e) => error!("version error"),
    }

    let rx_gains = [
        RxGain::DB18,
        RxGain::DB23,
        RxGain::DB33,
        RxGain::DB38,
        RxGain::DB43,
        RxGain::DB48,
    ];
    let mut rx_gain_index = 5;
    {
        let rx_gain = rx_gains[rx_gain_index];
        mfrc522.set_antenna_gain(rx_gain).unwrap();
        // info!("Initial rx gain: {rx_gain:?}");
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
                Err(_e) => {
                    warn!("Select error");
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
            info!("UID: {:?}", current_present_uid);
            led.set_level(current_present_uid.is_none().into());
        }

        // Update the state
        if let Some(uid) = current_uid {
            previous_uid = Some(PreviousUid {
                uid,
                last_seen: now,
            });
        }
        uid_last_updated = now;

        let button_fut = button.wait_for_rising_edge();
        pin_mut!(button_fut);
        match select(button_fut, Timer::after(poll_interval)).await {
            Either::Left(_) => {
                rx_gain_index += 1;
                if rx_gain_index == rx_gains.len() {
                    rx_gain_index = 0;
                }
                {
                    let rx_gain = rx_gains[rx_gain_index];
                    mfrc522.set_antenna_gain(rx_gain).unwrap();
                    info!("Set rx gain to ");
                }
            }
            Either::Right(_) => {}
        }
    }
}
