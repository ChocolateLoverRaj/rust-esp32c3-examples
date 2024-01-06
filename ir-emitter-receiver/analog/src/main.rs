use esp_idf_hal::{adc, delay::FreeRtos, gpio::PinDriver, peripherals::Peripherals, task};
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

    let mut adc_pin =
        adc::AdcChannelDriver::<{ adc::attenuation::DB_11 }, _>::new(peripherals.pins.gpio5)
            .unwrap();

    let mut adc_driver = adc::AdcDriver::new(
        peripherals.adc2,
        &adc::config::Config::new().calibration(true),
    )
    .unwrap();

    // Initialize Pin 8 as an output to drive the LED
    let mut led_pin = PinDriver::output(peripherals.pins.gpio8).unwrap();

    let mut count = 0;
    loop {
        let millivolts = adc_driver.read(&mut adc_pin).unwrap();
        println!("[{}] Measured voltage: {:?}mV", count, millivolts);
        let threshold: u16 = 50;
        let is_receiving_light = millivolts >= threshold;
        if is_receiving_light {
            led_pin.set_low().unwrap();
        } else {
            led_pin.set_high().unwrap();
        }
        FreeRtos::delay_ms(100);
        count += 1;
    }
}
