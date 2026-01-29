#![no_std]
#![no_main]

use defmt::info;
use embassy_executor::Spawner;
use esp_backtrace as _;
use esp_hal::{
    i2c::{self, master::I2c},
    interrupt::software::SoftwareInterruptControl,
    time::Rate,
    timer::timg::TimerGroup,
};
use esp_println as _;

esp_bootloader_esp_idf::esp_app_desc!();

#[esp_rtos::main]
async fn main(spawner: Spawner) {
    let _ = spawner;

    let p = esp_hal::init(Default::default());

    // Needed for esp_rtos
    let timg0 = TimerGroup::new(p.TIMG0);
    let software_interrupt = SoftwareInterruptControl::new(p.SW_INTERRUPT);
    esp_rtos::start(timg0.timer0, software_interrupt.software_interrupt0);

    info!("Hello from a Rust no_std environment with esp_rtos (basically embassy for ESP32).");

    let mut i2c = I2c::new(
        p.I2C0,
        i2c::master::Config::default().with_frequency(Rate::from_khz(400)),
    )
    .unwrap()
    .with_scl(p.GPIO5)
    .with_sda(p.GPIO6)
    .into_async();

    let mut buffer = [Default::default(); 4];
    i2c.write_read_async(0x20, &[10, 20], &mut buffer)
        .await
        .unwrap();
    info!("buffer: {}", buffer);
}
