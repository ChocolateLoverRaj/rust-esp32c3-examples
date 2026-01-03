#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embassy_stm32::{
    gpio::OutputType,
    time::hz,
    timer::{
        low_level::CountingMode,
        simple_pwm::{PwmPin, SimplePwm},
    },
};
use embassy_time::{Duration, TICK_HZ, Timer};
use {defmt_rtt as _, panic_probe as _};

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_stm32::init(Default::default());
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
    let period = Duration::from_secs(1);
    let delay = period / ch1.max_duty_cycle() as u32;
    info!("Delay: 1s / {} * {})", TICK_HZ, delay.as_ticks());
    loop {
        let mut new_duty_cycle = ch1.current_duty_cycle() + 1;
        if new_duty_cycle == ch1.max_duty_cycle() {
            new_duty_cycle = 0;
        }
        ch1.set_duty_cycle(new_duty_cycle);
        Timer::after(delay).await;
    }
}
