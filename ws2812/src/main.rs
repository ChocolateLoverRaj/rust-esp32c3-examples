#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use esp_backtrace as _;
use esp_hal::{
    interrupt::software::SoftwareInterruptControl,
    spi::{
        Mode,
        master::{Config, Spi},
    },
    time::Rate,
    timer::timg::TimerGroup,
};
use esp_println as _;
use smart_leds::{RGB8, SmartLedsWriteAsync, brightness};
use ws2812_async::{Grb, Ws2812};

esp_bootloader_esp_idf::esp_app_desc!();

#[esp_rtos::main]
async fn main(spawner: Spawner) {
    let _ = spawner;

    let peripherals = esp_hal::init(Default::default());

    // Needed for esp_rtos
    let timg0 = TimerGroup::new(peripherals.TIMG0);
    let software_interrupt = SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    esp_rtos::start(timg0.timer0, software_interrupt.software_interrupt0);

    let spi = Spi::new(
        peripherals.SPI2,
        Config::default()
            .with_frequency(Rate::from_hz(3_800_000))
            .with_mode(Mode::_0),
    )
    .unwrap()
    .with_mosi(peripherals.GPIO0)
    .into_async();

    // From the example at https://github.com/kalkyl/ws2812-async
    /// Change this to the number of LEDs that you have
    const NUM_LEDS: usize = 14;
    /// Change this to what order your specific LEDs have
    /// Here it says GRB: https://cdn-shop.adafruit.com/datasheets/WS2812.pdf
    type C = Grb;
    let mut ws = Ws2812::<_, C, { 12 * NUM_LEDS }>::new(spi);

    let mut data = [RGB8::default(); NUM_LEDS];

    loop {
        for j in 0..(256 * 5) {
            for i in 0..NUM_LEDS {
                data[i] = wheel((((i * 256) as u16 / NUM_LEDS as u16 + j as u16) & 255) as u8);
            }
            ws.write(brightness(data.iter().cloned(), 32)).await.ok();
            Timer::after(Duration::from_millis(5)).await;
        }
    }
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
