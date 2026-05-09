#![no_std]
#![no_main]

mod rotary_encoder;

use crate::rotary_encoder::{Direction, RotaryEncoder, State};
use embassy_executor::Spawner;
use embassy_futures::{join::join, select::select};
use embassy_stm32::{
    exti::ExtiInput,
    gpio::{Level, Output, OutputType, Pull, Speed},
    time::hz,
    timer::{
        low_level::CountingMode,
        simple_pwm::{PwmPin, SimplePwm},
    },
};
use {defmt_rtt as _, panic_probe as _};

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_stm32::init(Default::default());

    join(
        async {
            let mut led = Output::new(p.PC13, Level::High, Speed::Low);
            let mut sw = ExtiInput::new(p.PB9, p.EXTI9, Pull::Up);
            loop {
                sw.wait_for_falling_edge().await;
                led.toggle();
            }
        },
        async {
            let mut dt = ExtiInput::new(p.PB8, p.EXTI8, Pull::Up);
            let mut clk = ExtiInput::new(p.PB7, p.EXTI7, Pull::Up);
            let mut pwm = SimplePwm::new(
                p.TIM1,
                Some(PwmPin::new(p.PA8, OutputType::PushPull)),
                None,
                None,
                None,
                hz(2000),
                CountingMode::default(),
            );
            let mut ch1 = pwm.ch1();
            ch1.enable();
            let steps_until_on = 50;
            let mut numerator = steps_until_on / 2;
            ch1.set_duty_cycle_fraction(numerator, steps_until_on);

            let mut rotary_encoder = RotaryEncoder::new(State {
                dt: dt.is_low(),
                clk: clk.is_low(),
            });
            loop {
                select(dt.wait_for_any_edge(), clk.wait_for_any_edge()).await;
                if let Some(direction) = rotary_encoder.process_data(State {
                    dt: dt.is_low(),
                    clk: clk.is_low(),
                }) {
                    numerator = numerator
                        .saturating_add_signed(match direction {
                            Direction::Clockwise => 1,
                            Direction::CounterClockwise => -1,
                        })
                        .min(steps_until_on);
                    ch1.set_duty_cycle_fraction(numerator, steps_until_on);
                }
            }
        },
    )
    .await;
}
