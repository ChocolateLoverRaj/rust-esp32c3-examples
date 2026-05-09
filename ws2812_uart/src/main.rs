#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use esp_backtrace as _;
use esp_hal::{
    gpio::{Level, Output, OutputConfig},
    interrupt::software::SoftwareInterruptControl,
    timer::timg::TimerGroup,
    uart::{self, Uart},
};
use smart_leds::{RGB8, SmartLedsWriteAsync};
use ws2812_uart::Ws2812;

esp_bootloader_esp_idf::esp_app_desc!();

#[esp_rtos::main]
async fn main(spawner: Spawner) {
    let _ = spawner;

    let peripherals = esp_hal::init(Default::default());

    // Needed for esp_rtos
    let timg0 = TimerGroup::new(peripherals.TIMG0);
    let software_interrupt = SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    esp_rtos::start(timg0.timer0, software_interrupt.software_interrupt0);

    let uart = Uart::new(
        peripherals.UART0,
        uart::Config::default().with_baudrate(3_750_000),
    )
    .unwrap()
    .with_tx(
        Output::new(peripherals.GPIO2, Level::High, OutputConfig::default())
            .into_peripheral_output()
            .with_output_inverter(true),
    )
    .into_async();
    let mut uart = Ws2812::<_, ws2812_uart::device::Ws2812>::new(uart);

    const N_LEDS: usize = 64;
    let mut pixels = [RGB8::default(); N_LEDS];
    loop {
        for color in [
            RGB8::new(25, 0, 0),
            RGB8::new(0, 25, 0),
            RGB8::new(0, 0, 25),
        ] {
            for i in 0..N_LEDS {
                pixels[i] = color;
                uart.write(pixels).await.unwrap();
                Timer::after(Duration::from_millis(50)).await;
                pixels[i] = RGB8::default();
            }
        }
    }
}
