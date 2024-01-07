use esp_idf_hal::{
    delay::FreeRtos,
    gpio::{InterruptType, PinDriver, Pull},
    peripherals::Peripherals,
    task,
};
use esp_idf_svc::systime::EspSystemTime;
use esp_idf_sys as _; // If using the `binstart` feature of `esp-idf-sys`, always keep this module imported
use esp_println::println;

fn main() {
    task::block_on(main_async());
}

async fn main_async() {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_sys::link_patches();

    let peripherals = Peripherals::take().unwrap();

    // Instantiate and Create Handle for trigger output & echo input
    let mut trigger_pin = PinDriver::output(peripherals.pins.gpio21).unwrap();
    let mut echo_pin = PinDriver::input(peripherals.pins.gpio20).unwrap();
    echo_pin.set_pull(Pull::Down).unwrap();
    echo_pin.set_interrupt_type(InterruptType::AnyEdge).unwrap();
    echo_pin.enable_interrupt().unwrap();

    loop {
        // Send a 10us pulse to the trigger pin to start the measurement
        trigger_pin.set_high().unwrap();
        FreeRtos::delay_us(10);
        trigger_pin.set_low().unwrap();

        // Wait for the echo pin to go high (start of pulse)
        echo_pin.wait_for_high().await.unwrap();

        // Measure the duration of the echo pulse (in microseconds)
        let start_time = EspSystemTime {}.now().as_micros();
        echo_pin.wait_for_low().await.unwrap();
        let end_time = EspSystemTime {}.now().as_micros();

        // Calculate the duration of the echo pulse in microseconds
        let pulse_duration = end_time - start_time;

        // Calculate the distance based on the speed of sound (approximately 343 m/s)
        // Distance in centimeters: duration * speed_of_sound / 2 (since the signal goes to the object and back)
        let distance_cm = (pulse_duration as f32 * 0.0343) / 2.0;

        println!("Distance: {}cm", distance_cm);

        FreeRtos::delay_ms(300);
    }
}
