#![no_std]
#![no_main]

use core::mem;

use defmt::info;
use embassy_embedded_hal::{adapter::BlockingAsync, shared_bus::asynch::i2c::I2cDeviceWithConfig};
use embassy_executor::Spawner;
use embassy_futures::{
    join::{join_array, join3},
    select::select_array,
};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, mutex::Mutex};
use embassy_time::{Delay, Instant, Timer};
use embedded_hal::digital::PinState;
use embedded_hal_async::digital::OutputPin;
use esp_backtrace as _;
use esp_hal::{
    gpio::{Input, InputConfig, Level, Output, OutputConfig, Pull},
    i2c::{self},
    interrupt::software::SoftwareInterruptControl,
    time::Rate,
    timer::timg::TimerGroup,
};
use esp_println as _;
use mcp23017_controller::Mcp23017;
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

    let mut mcp23017 = Mcp23017::new(
        I2cDeviceWithConfig::new(&i2c, config),
        [false, false, false],
        BlockingAsync::new(Output::new(p.GPIO8, Level::High, OutputConfig::default())),
        Input::new(p.GPIO1, InputConfig::default().with_pull(Pull::Up)),
        Delay,
    );
    let (runner, pins) = mcp23017.run();
    join3(
        async { runner.await.unwrap() },
        join_array(
            [(pins.A0, "A0"), (pins.A1, "A1")].map(async |(pin, _name)| {
                let mut state = PinState::High;
                let mut pin = pin.into_output(state).await;
                loop {
                    Timer::after_secs(1).await;
                    state = !state;
                    pin.set_state(state).await;
                }
            }),
        ),
        async {
            let mut sw = pins.B1.into_watch(true).await;
            let mut dt = pins.B2.into_watch(true).await;
            let mut clk = pins.B3.into_watch(true).await;
            let mut last_read = Instant::now();
            let mut rotary_encoder = RotaryEncoder::new(RotaryPinsState {
                clk: clk.state().await == PinState::Low,
                dt: dt.state().await == PinState::Low,
            });
            let mut position = 0_i64;
            loop {
                select_array([sw.watch(), dt.watch(), clk.watch()]).await;
                let sw_state = sw.state().await;
                let dt_state = dt.state().await;
                let clk_state = clk.state().await;
                let now = Instant::now();
                let last_read = mem::replace(&mut last_read, now);
                let rotary_pins = RotaryPinsState {
                    dt: dt_state == PinState::Low,
                    clk: clk_state == PinState::Low,
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
                    u8::from(sw_state == PinState::Low),
                    u8::from(dt_state == PinState::Low),
                    u8::from(clk_state == PinState::Low),
                    direction,
                    position,
                    (now - last_read).as_micros(),
                );
            }
        },
        // join_array(
        //     [(pin_b_1, "B1"), (pin_b_2, "B2"), (pin_b_3, "B3")].map(async |(pin, name)| {
        //         let mut pin = pin.into_watch(true).await;
        //         loop {
        //             let state = pin.state().await;
        //             info!("{} is {}", name, Debug2Format(&state));
        //             pin.watch().await;
        //         }
        //     }),
        // ),
        // async {
        //     let mut state = PinState::Low;
        //     let mut pin = pin_a_1.into_output(state).await;
        //     loop {
        //         Timer::after_secs(1).await;
        //         state = !state;
        //         pin.set_state(state).await;
        //     }
        // },
        // join_array(
        //     [(pin_b_1, "B1"), (pin_b_2, "B2"), (pin_b_3, "B3")].map(async |(pin, name)| {
        //         let mut pin = pin.into_input(true).await;
        //         // let is_high = pin.is_high().await.unwrap();
        //         // info!("{}: {}", name, Debug2Format(&PinState::from(is_high)));
        //         // loop {
        //         //     pin.wait_for_low().await;
        //         //     info!("{} is low", name);
        //         //     pin.wait_for_high().await;
        //         //     info!("{} is high", name);
        //         // }
        //         // loop {
        //         //     let is_high = pin.is_high().await.unwrap();
        //         //     info!("{}: {}", name, Debug2Format(&PinState::from(is_high)));
        //         //     pin.wait_for_any_edge().await;
        //         // }

        //         loop {
        //             pin.wait_for_falling_edge().await;
        //             info!("{} falling edge", name);
        //             pin.wait_for_rising_edge().await;
        //             info!("{} rising edge", name);
        //         }
        //     }),
        // ),
    )
    .await;
}
