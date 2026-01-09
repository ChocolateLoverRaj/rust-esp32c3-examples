#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use esp_backtrace as _;
use esp_hal::{
    interrupt::software::SoftwareInterruptControl, rmt::Rmt, time::Rate, timer::timg::TimerGroup,
};
use esp_hal_smartled::{SmartLedsAdapterAsync, buffer_size_async, smart_led_buffer};
use esp_println as _;
use smart_leds::{RGB8, SmartLedsWriteAsync};

esp_bootloader_esp_idf::esp_app_desc!();

#[esp_rtos::main]
async fn main(spawner: Spawner) {
    let _ = spawner;

    let p = esp_hal::init(Default::default());

    // Needed for esp_rtos
    let timg0 = TimerGroup::new(p.TIMG0);
    let software_interrupt = SoftwareInterruptControl::new(p.SW_INTERRUPT);
    esp_rtos::start(timg0.timer0, software_interrupt.software_interrupt0);
    const N_LEDS: usize = 64;
    let mut buffer = smart_led_buffer!(buffer_size_async(N_LEDS));
    let mut leds_adapter = {
        SmartLedsAdapterAsync::new(
            Rmt::new(p.RMT, Rate::from_mhz(80))
                .unwrap()
                .into_async()
                .channel0,
            p.GPIO2,
            &mut buffer,
        )
    };
    let mut leds = [RGB8::default(); N_LEDS];
    loop {
        for offset in 0..N_LEDS {
            for (i, led) in leds.iter_mut().enumerate() {
                let brightness = 15;
                let adjusted_i = {
                    let adjusted_i = i + offset;
                    if adjusted_i >= N_LEDS {
                        adjusted_i - N_LEDS
                    } else {
                        adjusted_i
                    }
                };
                *led = RGB8::new(
                    ((1.0 - adjusted_i as f64 / (N_LEDS - 1) as f64) * brightness as f64) as u8,
                    0,
                    (adjusted_i as f64 / (N_LEDS - 1) as f64 * brightness as f64) as u8,
                );
            }
            leds_adapter.write(leds).await.unwrap();
            Timer::after(Duration::from_millis(25)).await;
        }
    }
}
