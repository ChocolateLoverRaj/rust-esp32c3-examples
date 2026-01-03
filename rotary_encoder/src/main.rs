#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embassy_futures::{join::join, select::select};
use embassy_stm32::{
    exti::{self, ExtiInput},
    gpio::{Input, Level, Output, OutputType, Pull, Speed},
    time::hz,
    timer::{
        low_level::CountingMode,
        simple_pwm::{PwmPin, SimplePwm},
    },
};
use embassy_time::{Duration, TICK_HZ, Timer};
use embedded_hal::digital::v2::InputPin;
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

            #[derive(Debug, Format, Clone, Copy, PartialEq, Eq)]
            struct State {
                dt: bool,
                clk: bool,
            }

            let mut prev_state = State {
                dt: dt.is_low(),
                clk: clk.is_low(),
            };

            #[derive(Debug, Format, Clone, Copy, PartialEq, Eq)]
            enum RotaryPin {
                Dt,
                Clock,
            }
            impl RotaryPin {
                /// Returns +1 if clockwise and -1 if counter-clockwise
                pub fn leading_direction(&self) -> i8 {
                    match self {
                        Self::Dt => -1,
                        Self::Clock => 1,
                    }
                }
            }
            let mut leading_pin_option = None;
            let mut position = 0_i64;

            info!("initial state: {}", prev_state);
            loop {
                select(dt.wait_for_any_edge(), clk.wait_for_any_edge()).await;
                let state = State {
                    dt: dt.is_low(),
                    clk: clk.is_low(),
                };
                if state != prev_state {
                    trace!("new state: {}", state);
                    let clk_changed = state.clk != prev_state.clk;
                    let dt_changed = state.dt != prev_state.dt;
                    let changed_pin = match (clk_changed, dt_changed) {
                        (true, false) => Some(RotaryPin::Clock),
                        (false, true) => Some(RotaryPin::Dt),
                        _ => None,
                    };
                    if let Some(changed_pin) = changed_pin {
                        let change = if let Some(leading_pin) = leading_pin_option {
                            let change = if changed_pin != leading_pin {
                                trace!("{} caught up", changed_pin);
                                leading_pin.leading_direction()
                            } else {
                                trace!("{} moved back. direction changed", changed_pin);
                                -leading_pin.leading_direction()
                            };
                            leading_pin_option = None;
                            change
                        } else {
                            leading_pin_option = Some(changed_pin);
                            trace!("{} moved", changed_pin);
                            changed_pin.leading_direction()
                        };
                        position += i64::from(change);
                        info!("position: {}", position);
                    } else {
                        warn!(
                            "BOTH CHANGED! prev_state = {}. state = {}",
                            prev_state, state
                        );
                    }
                }
                prev_state = state;
            }

            // let mut clk_state = clk.is_low();
            // // info!("initial state: {}", state);
            // let mut position = 0_i64;
            // info!("initial position: {}", position);
            // loop {
            //     // select(dt.wait_for_any_edge(), clk.wait_for_any_edge()).await;
            //     clk.wait_for_any_edge().await;
            //     let new_clk_state = clk.is_low();
            //     if new_clk_state != clk_state {
            //         if new_clk_state {
            //             if dt.is_low() == new_clk_state {
            //                 position += 1;
            //             } else {
            //                 position -= 1;
            //             }
            //             info!("new position: {}", position);
            //         }
            //         clk_state = new_clk_state;
            //     }
            //     // let direction = encoder.update(dt.is_low(), clk.is_low());
            //     // match direction {
            //     //     Direction::Clockwise => {
            //     //         info!("direction: clockwise");
            //     //     }
            //     //     Direction::Anticlockwise => {
            //     //         info!("direction: anticlockwise");
            //     //     }
            //     //     Direction::None => {
            //     //         info!("direction: none");
            //     //     }
            //     // }
            // }

            // let mut pwm = SimplePwm::new(
            //     p.TIM1,
            //     Some(PwmPin::new(p.PA8, OutputType::PushPull)),
            //     None,
            //     None,
            //     None,
            //     hz(2000),
            //     CountingMode::default(),
            // );
            // let mut ch1 = pwm.ch1();
            // ch1.enable();
            // let period = Duration::from_secs(1);
            // let delay = period / ch1.max_duty_cycle() as u32;
            // info!("Delay: 1s / {} * {})", TICK_HZ, delay.as_ticks());
            // loop {
            //     let mut new_duty_cycle = ch1.current_duty_cycle() + 1;
            //     if new_duty_cycle == ch1.max_duty_cycle() {
            //         new_duty_cycle = 0;
            //     }
            //     ch1.set_duty_cycle(new_duty_cycle);
            //     Timer::after(delay).await;
            // }
        },
    )
    .await;
}
