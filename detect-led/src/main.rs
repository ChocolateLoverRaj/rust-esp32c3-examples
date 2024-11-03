use std::{thread, time::Duration};

use esp_idf_hal::{
    gpio::{PinDriver, Pull},
    peripherals::Peripherals,
    task::block_on,
};
use esp_idf_sys as _;

fn main() {
    block_on(main_async());
}

async fn main_async() {
    // It is necessary to call this function once,
    // or else some patches to the runtime implemented by esp-idf-sys might not link properly.
    esp_idf_sys::link_patches();

    let peripherals = Peripherals::take().unwrap();
    let mut pin = PinDriver::input(peripherals.pins.gpio21).unwrap();
    pin.set_pull(Pull::Up).unwrap();

    if pin.is_low() {
        println!("LED is connected. Will blink");
        let mut pin = pin.into_output().unwrap();
        pin.set_high().unwrap();
        loop {
            thread::sleep(Duration::from_millis(375));
            pin.toggle().unwrap();
        }
    } else {
        println!("LED not connected. Will not blink");
    }
}
