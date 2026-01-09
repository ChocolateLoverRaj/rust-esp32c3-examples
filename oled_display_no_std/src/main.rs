#![no_std]
#![no_main]

use defmt::info;
use embassy_executor::Spawner;
use embassy_time::{Instant, Timer};
use embedded_graphics::{
    mono_font::{MonoTextStyleBuilder, iso_8859_16::FONT_10X20},
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::{PrimitiveStyleBuilder, Rectangle},
    text::{Baseline, Text},
};
use esp_backtrace as _;
use esp_hal::{
    i2c::master::{Config, I2c, SoftwareTimeout},
    interrupt::software::SoftwareInterruptControl,
    time::Rate,
    timer::timg::TimerGroup,
};
use esp_println as _;
use ssd1306::{I2CDisplayInterface, Ssd1306Async, prelude::*};

esp_bootloader_esp_idf::esp_app_desc!();

#[esp_rtos::main]
async fn main(spawner: Spawner) {
    let _ = spawner;

    let peripherals = esp_hal::init(Default::default());

    // Needed for esp_rtos
    let timg0 = TimerGroup::new(peripherals.TIMG0);
    let software_interrupt = SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    esp_rtos::start(timg0.timer0, software_interrupt.software_interrupt0);

    let i2c = I2c::new(
        peripherals.I2C0,
        Config::default()
            .with_frequency(Rate::from_khz(400))
            .with_software_timeout(SoftwareTimeout::PerByte(
                esp_hal::time::Duration::from_millis(1),
            )),
    )
    .unwrap()
    .with_scl(peripherals.GPIO0)
    .with_sda(peripherals.GPIO1)
    .into_async();

    let interface = I2CDisplayInterface::new(i2c);
    let mut display = Ssd1306Async::new(interface, DisplaySize128x64, DisplayRotation::Rotate0)
        .into_buffered_graphics_mode();
    display.init().await.unwrap();

    let text_style = MonoTextStyleBuilder::new()
        .font(&FONT_10X20)
        .text_color(BinaryColor::On)
        .build();

    Text::with_baseline(
        "Start Game!\nLanguage\nPlayers\n",
        Point::zero(),
        text_style,
        Baseline::Top,
    )
    .draw(&mut display)
    .unwrap();
    display.flush().await.unwrap();

    // // Do an animation
    // enum State {
    //     Growing,
    //     Shrinking,
    // }
    // let mut state = State::Growing;
    // let mut width = 0;
    // loop {
    //     let before = Instant::now();
    //     display.clear(BinaryColor::Off).unwrap();
    //     Rectangle::new(
    //         Point::new(
    //             match state {
    //                 State::Growing => 0,
    //                 State::Shrinking => (display.size().width - width) as i32,
    //             },
    //             0,
    //         ),
    //         Size::new(width, display.size().height),
    //     )
    //     .into_styled(
    //         PrimitiveStyleBuilder::new()
    //             .fill_color(BinaryColor::On)
    //             .build(),
    //     )
    //     .draw(&mut display)
    //     .unwrap();
    //     display.flush().await.unwrap();
    //     info!("Draw time: {} us", before.elapsed().as_micros());
    //     // Timer::after(embassy_time::Duration::from_nanos(7_812_500)).await;
    //     match state {
    //         State::Growing => {
    //             width += 1;
    //             if width == display.size().width {
    //                 state = State::Shrinking;
    //             }
    //         }
    //         State::Shrinking => {
    //             width -= 1;
    //             if width == 0 {
    //                 state = State::Growing;
    //             }
    //         }
    //     };
    // }

    // display.flush().await.unwrap();
    // let mut invert = false;
    // loop {
    //     Timer::after(embassy_time::Duration::from_secs(5)).await;
    //     invert = !invert;
    //     display.set_invert(invert).await.unwrap();
    // }
}
