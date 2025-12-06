#![no_std]
#![no_main]

use core::future::pending;

use embassy_executor::Spawner;
use esp_backtrace as _;
use esp_println::println;

esp_bootloader_esp_idf::esp_app_desc!();

#[esp_rtos::main]
async fn main(spawner: Spawner) {
    let _ = spawner;

    let _peripherals = esp_hal::init(Default::default());

    println!("Hello cool Rust ESP32 no_std ecosystem!");

    pending().await
}
