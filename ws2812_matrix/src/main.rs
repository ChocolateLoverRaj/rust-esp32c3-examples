#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use embedded_graphics::{
    mono_font::{MonoTextStyle, iso_8859_16::FONT_4X6},
    pixelcolor::Rgb888,
    prelude::*,
    primitives::{PrimitiveStyleBuilder, Rectangle},
    text::{Baseline, Text, renderer::TextRenderer},
};
use esp_backtrace as _;
use esp_hal::{
    interrupt::software::SoftwareInterruptControl, rmt::Rmt, time::Rate, timer::timg::TimerGroup,
};
use esp_hal_smartled::{SmartLedsAdapter, buffer_size_async, smart_led_buffer};
use esp_println as _;
use smart_leds_matrix::{SmartLedMatrix, layout::Rectangular};

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
    let mut buffer = smart_led_buffer!(N_LEDS);
    let leds_adapter = {
        SmartLedsAdapter::new(
            Rmt::new(p.RMT, Rate::from_mhz(80)).unwrap().channel0,
            p.GPIO7,
            &mut buffer,
        )
    };
    let mut matrix = SmartLedMatrix::<_, _, N_LEDS>::new(leds_adapter, Rectangular::new(8, 8));

    matrix.set_brightness(5);
    // Create a new character style
    let style = MonoTextStyle::new(&FONT_4X6, Rgb888::RED);

    let mut text = Text::with_baseline("3.14159265", Point::zero(), style, Baseline::Top);
    let pixels_to_scroll = text
        .bounding_box()
        .size
        .width
        .checked_sub(matrix.bounding_box().size.width);
    let mut scroll_x = 0_u32;
    loop {
        text.position.x = 0 - scroll_x as i32;
        matrix.clear(Rgb888::BLACK);
        text.draw(&mut matrix).unwrap();
        matrix.flush().unwrap();
        if let Some(pixels_to_scroll) = pixels_to_scroll {
            if scroll_x == 0 {
                Timer::after(Duration::from_secs(1)).await;
                scroll_x += 1;
            } else if scroll_x == pixels_to_scroll {
                Timer::after(Duration::from_secs(1)).await;
                scroll_x = 0;
            } else {
                Timer::after(Duration::from_millis(150)).await;
                scroll_x += 1;
            }
        } else {
            break;
        }
    }
}
