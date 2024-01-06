use esp_idf_hal::{
    gpio::{IOPin, PinDriver, Pull},
    peripherals::Peripherals,
    task,
};
use esp_idf_sys as _; // If using the `binstart` feature of `esp-idf-sys`, always keep this module imported
use esp_println::println;

fn main() {
    task::block_on(main_async());
}

async fn main_async() {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_sys::link_patches();

    // Get all the peripherals
    let peripherals = Peripherals::take().unwrap();

    let mut receiver_pin = PinDriver::input(peripherals.pins.gpio5.downgrade()).unwrap();
    receiver_pin.set_pull(Pull::Down).unwrap();

    // Initialize Pin 8 as an output to drive the LED
    let mut led_pin = PinDriver::output(peripherals.pins.gpio8).unwrap();

    let mut previous = None::<bool>;
    loop {
        let is_receiving_light = receiver_pin.is_low();
        if is_receiving_light {
            led_pin.set_low().unwrap();
        } else {
            led_pin.set_high().unwrap();
        }
        if previous.map_or(true, |previous| is_receiving_light != previous) {
            println!("Receiver receiving IR light: {}", is_receiving_light);
        }
        previous = Some(is_receiving_light);
    }
}
