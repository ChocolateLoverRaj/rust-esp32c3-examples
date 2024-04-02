use async_vl53l0x::AsyncVL53L0x;
use esp_idf_hal::prelude::*;
use esp_idf_hal::{
    i2c::{I2cConfig, I2cDriver},
    task,
};
use esp_idf_sys as _;
// If using the `binstart` feature of `esp-idf-sys`, always keep this module imported
use esp_println::println;

mod async_vl53l0x;

fn main() {
    task::block_on(main_async());
}

async fn main_async() {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_sys::link_patches();

    // Get all the peripherals
    let peripherals = Peripherals::take().unwrap();

    // You can actually use any GPIO pin for I2C
    let sda = peripherals.pins.gpio2;
    let scl = peripherals.pins.gpio3;

    let config = I2cConfig::new().baudrate(1000.kHz().into());
    let i2c = I2cDriver::new(peripherals.i2c0, sda, scl, &config).unwrap();

    match AsyncVL53L0x::new_with_gpio1(i2c, peripherals.pins.gpio1) {
        Ok(mut async_vl53l0x) => {
            async_vl53l0x
                .vl53l0x
                .set_measurement_timing_budget(20_000)
                .unwrap();
            async_vl53l0x.vl53l0x.start_continuous(0).unwrap();

            loop {
                match async_vl53l0x.read_range_mm_async().await {
                    Ok(distance) => {
                        // It seems like if the light does not bounce back, it will report a distance of 8190
                        // Sometimes it keeps switching between ~1250 and 8190, and if 8190 is continuously printed, it's hard to read the actual distance.
                        if distance != 8190 {
                            println!("Distance: {}mm", distance);
                        }
                    }
                    Err(e) => {
                        println!("Error reading distance: {:#?}", e);
                    }
                }
            }
        }
        Err(e) => {
            println!("Error creating VL53L0x: {:#?}", e);
        }
    }
}
