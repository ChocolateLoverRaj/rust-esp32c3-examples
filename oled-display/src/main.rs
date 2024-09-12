use std::{
    thread::{self, sleep},
    time::Duration,
};

use embedded_graphics::{
    image::Image,
    mono_font::{
        ascii::{FONT_6X10, FONT_8X13, FONT_9X15},
        iso_8859_10::FONT_6X13,
        MonoTextStyleBuilder,
    },
    pixelcolor::{self, BinaryColor, Rgb888},
    prelude::*,
    text::{Baseline, Text},
};
use esp_idf_hal::{
    gpio::{DriveStrength, InterruptType, PinDriver, Pull},
    peripherals::Peripherals,
    task::block_on,
};
use esp_idf_svc::nvs::{EspNvs, EspNvsPartition, NvsDefault};
use esp_idf_sys as _;
use ssd1306::{mode::BufferedGraphicsMode, prelude::*, I2CDisplayInterface, Ssd1306};
use tinytga::Tga;

fn main() -> anyhow::Result<()> {
    block_on(main_async())
}

async fn main_async() -> anyhow::Result<()> {
    // It is necessary to call this function once,
    // or else some patches to the runtime implemented by esp-idf-sys might not link properly.
    esp_idf_sys::link_patches();

    let peripherals = Peripherals::take()?;
    let i2c = esp_idf_svc::hal::i2c::I2cDriver::new(
        peripherals.i2c0,
        peripherals.pins.gpio8,
        peripherals.pins.gpio9,
        &Default::default(),
    )?;
    let interface = I2CDisplayInterface::new(i2c);
    let mut display = Ssd1306::new(interface, DisplaySize128x64, DisplayRotation::Rotate0)
        .into_buffered_graphics_mode();
    display.init().unwrap();
    let data = include_bytes!("../rust.tga");
    let tga: Tga<BinaryColor> = Tga::from_slice(data).unwrap();
    let image0 = Image::new(&tga, Point::zero());
    image0.draw(&mut display).unwrap();
    let image1 = Image::new(&tga, Point::new(64, 0));
    image1.draw(&mut display).unwrap();
    display.set_brightness(Brightness::custom(1, 1)).unwrap();
    let mut invert = false;
    loop {
        display.flush().unwrap();
        sleep(Duration::from_secs_f64(5.0));
        invert = !invert;
        display.set_invert(invert).unwrap();
    }
}
