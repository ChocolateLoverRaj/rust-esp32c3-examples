#![no_std]
#![no_main]

use core::mem;

use controller::Mcp23017;
use defmt::info;
use embassy_embedded_hal::shared_bus::asynch::i2c::I2cDeviceWithConfig;
use embassy_executor::Spawner;
use embassy_futures::select::select_array;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, mutex::Mutex};
use embassy_time::{Delay, Instant};
use embedded_hal::digital::PinState;
use esp_backtrace as _;
use esp_hal::{
    gpio::{Input, InputConfig, Level, Output, OutputConfig, Pull},
    i2c::{self},
    interrupt::software::SoftwareInterruptControl,
    time::Rate,
    timer::timg::TimerGroup,
};
use esp_println as _;
use pure_rotary_encoder::{Direction, RotaryEncoder, RotaryPinsState};

esp_bootloader_esp_idf::esp_app_desc!();

#[esp_rtos::main]
async fn main(spawner: Spawner) {
    let _ = spawner;

    let p = esp_hal::init(Default::default());

    // Needed for esp_rtos
    let timg0 = TimerGroup::new(p.TIMG0);
    let software_interrupt = SoftwareInterruptControl::new(p.SW_INTERRUPT);
    esp_rtos::start(timg0.timer0, software_interrupt.software_interrupt0);

    info!("Hello from a Rust no_std environment with esp_rtos (basically embassy for ESP32).");

    let config = i2c::master::Config::default().with_frequency(Rate::from_khz(400));
    let i2c = Mutex::<CriticalSectionRawMutex, _>::new(
        i2c::master::I2c::new(p.I2C0, Default::default())
            .unwrap()
            .with_scl(p.GPIO5)
            .with_sda(p.GPIO6)
            .into_async(),
    );

    let mcp23017 = Mcp23017::new(
        Output::new(p.GPIO8, Level::High, OutputConfig::default()),
        I2cDeviceWithConfig::new(&i2c, config),
        [false, false, false],
        Input::new(p.GPIO1, InputConfig::default().with_pull(Pull::Up)),
        &mut Delay,
    )
    .await
    .unwrap();
    let mut sw = mcp23017.input(9, true).await.unwrap();
    let mut dt = mcp23017.input(10, true).await.unwrap();
    let mut clk = mcp23017.input(11, true).await.unwrap();
    let mut last_read = Instant::now();
    let mut rotary_encoder = RotaryEncoder::new(RotaryPinsState {
        clk: clk.last_known_state() == PinState::Low,
        dt: dt.last_known_state() == PinState::Low,
    });
    let mut position = 0_i64;
    loop {
        select_array([
            sw.wait_for_change(),
            dt.wait_for_change(),
            clk.wait_for_change(),
        ])
        .await
        .0
        .unwrap();

        let now = Instant::now();
        let last_read = mem::replace(&mut last_read, now);
        let rotary_pins = RotaryPinsState {
            dt: dt.last_known_state() == PinState::Low,
            clk: clk.last_known_state() == PinState::Low,
        };
        let direction = rotary_encoder.process_data(rotary_pins);
        if let Some(direction) = direction {
            position += match direction {
                Direction::Clockwise => 1,
                Direction::CounterClockwise => -1,
            };
        }
        info!(
            "sw = {}, dt = {}, clk = {}, direction = {}, position = {}, duration = {} us",
            u8::from(sw.last_known_state() == PinState::Low),
            u8::from(dt.last_known_state() == PinState::Low),
            u8::from(clk.last_known_state() == PinState::Low),
            direction,
            position,
            (now - last_read).as_micros(),
        );
    }
}
