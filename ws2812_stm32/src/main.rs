#![no_std]
#![no_main]

mod rotary_encoder;

use core::{iter::repeat, u8};

use embassy_executor::Spawner;
use embassy_futures::select::select;
use embassy_stm32::{
    gpio::{Level, Output, Speed},
    spi::{Config, Spi},
    time::hz,
};
use embassy_time::{Duration, Timer};
use smart_leds::{RGB8, SmartLedsWriteAsync, brightness};
use ws2812_async::{Grb, Ws2812};
use {defmt_rtt as _, panic_probe as _};

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_stm32::init(Default::default());

    let spi = Spi::new_txonly_nosck(p.SPI1, p.PA7, p.DMA1_CH3, {
        let mut config = Config::default();
        config.frequency = hz(3_800_000);
        config
    });

    const NUM_LEDS: usize = 64;
    let mut ws: Ws2812<_, Grb, NUM_LEDS> = Ws2812::new(spi);
    let mut built_in_led = Output::new(p.PC13, Level::Low, Speed::Low);
    loop {
        let _ = ws.write(repeat(RGB8::new(u8::MAX, u8::MAX, u8::MAX))).await;
        select(
            async {
                built_in_led.set_high();
                Timer::after_millis(100).await;
                built_in_led.set_low();
            },
            Timer::after_millis(500),
        )
        .await;
        built_in_led.set_low();
    }
    // // I'm using USB-C at 5V 3A
    // // 3000 mA
    // let total_max_current = 6000;
    // // 60 mA
    // let led_max_current = 60;

    // let full_power_leds = total_max_current / led_max_current;
    // let on_leds = data.len().min(full_power_leds);
    // loop {
    //     for on_leds in 1..=4 {
    //         data[..on_leds].fill(RGB8::new(255, 255, 255));
    //         data[on_leds..].fill(Default::default());
    //         for _ in 0..300 {
    //             ws.write(data).await.unwrap();
    //             Timer::after_millis(10).await;
    //         }
    //     }
    // }
    // let on_leds = 4;

    // Leave the rest of the LEDs off

    // loop {
    //     for j in 0..(256 * 5) {
    //         for i in 0..NUM_LEDS {
    //             data[i] = wheel((((i * 256) as u16 / NUM_LEDS as u16 + j as u16) & 255) as u8);
    //         }
    //         ws.write(brightness(data.iter().cloned(), 199)).await.ok();
    //         Timer::after(Duration::from_millis(5)).await;
    //     }
    // }
}

fn wheel(mut wheel_pos: u8) -> RGB8 {
    wheel_pos = 255 - wheel_pos;
    if wheel_pos < 85 {
        return (255 - wheel_pos * 3, 0, wheel_pos * 3).into();
    }
    if wheel_pos < 170 {
        wheel_pos -= 85;
        return (0, wheel_pos * 3, 255 - wheel_pos * 3).into();
    }
    wheel_pos -= 170;
    (wheel_pos * 3, 255 - wheel_pos * 3, 0).into()
}
