use esp_idf_hal::{
    adc::{
        self, attenuation, config::Config, Adc, AdcChannelDriver, AdcDriver, Atten11dB, Attenuated,
        ADC1,
    },
    delay::FreeRtos,
    gpio::{Gpio5, IOPin, InterruptType, PinDriver, Pull},
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

    println!("Starting 0-input\nThis application is a basic blinky program that turns an LED on and off every 1 second.\n");

    // Get all the peripherals
    let peripherals = Peripherals::take().unwrap();
    // Initialize Pin 8 as an output to drive the LED
    // let mut btn_pin = PinDriver::input(peripherals.pins.gpio5.downgrade()).unwrap();
    // btn_pin.set_pull(Pull::Down).unwrap();
    // btn_pin.set_interrupt_type(InterruptType::AnyEdge).unwrap();
    let mut adc_pin =
        adc::AdcChannelDriver::<{ adc::attenuation::DB_11 }, _>::new(peripherals.pins.gpio5)
            .unwrap();

    let mut adc_driver = adc::AdcDriver::new(
        peripherals.adc2,
        &adc::config::Config::new().calibration(true),
    )
    .unwrap();

    let mut led_pin = PinDriver::output(peripherals.pins.gpio8).unwrap();

    let mut count = 0;
    loop {
        let millivolts = adc_driver.read(&mut adc_pin).unwrap();
        println!("Measured voltage: {:?}mV", millivolts);
        let threshold: u16 = 50;
        let is_receiving_light = millivolts >= threshold;
        // let is_receiving_light = btn_pin.is_low();
        println!(
            "[{}] Receiver receiving IR light: {}",
            count, is_receiving_light
        );
        if is_receiving_light {
            led_pin.set_low().unwrap();
        } else {
            led_pin.set_high().unwrap();
        }
        FreeRtos::delay_ms(100);
        // btn_pin.wait_for_any_edge().await.unwrap();
        count += 1;
    }
}
