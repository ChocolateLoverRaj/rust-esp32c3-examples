use std::io::Read;

use embedded_hal::pwm::SetDutyCycle;
use esp_idf_hal::gpio::PinDriver;
use esp_idf_hal::ledc::*;
use esp_idf_hal::peripherals::Peripherals;
use esp_idf_hal::prelude::*;
use esp_idf_hal::task::block_on;
use esp_idf_sys::vTaskDelay;
use ir_remote::ir_signal::IrSignal;
use log::info;
use postcard::{take_from_bytes, Error};

fn main() -> anyhow::Result<()> {
    block_on(main_async())
}

async fn main_async() -> anyhow::Result<()> {
    esp_idf_hal::sys::link_patches();

    println!("Configuring output channel");

    let peripherals = Peripherals::take()?;
    let mut channel = LedcDriver::new(
        peripherals.ledc.channel0,
        LedcTimerDriver::new(
            peripherals.ledc.timer0,
            &config::TimerConfig::new().frequency(38.kHz().into()),
        )?,
        peripherals.pins.gpio0,
    )?;

    let mut internal_led = PinDriver::input_output(peripherals.pins.gpio8)?;
    internal_led.set_high()?;

    channel.disable()?;
    channel.set_duty_cycle_fraction(1, 2)?;

    let mut handle = std::io::stdin().lock();
    let mut buffer = [Default::default(); 1024];
    let mut data_len = Default::default();

    loop {
        let signal = {
            let (signal, leftover_len) = loop {
                match handle.read(&mut buffer[data_len..]) {
                    Ok(len) => match take_from_bytes::<IrSignal>(&buffer[..data_len + len]) {
                        Ok((signal, leftover)) => {
                            break Ok((signal, leftover.len()));
                        }
                        Err(e) => match e {
                            Error::DeserializeUnexpectedEnd => {}
                            _ => break Err(e),
                        },
                    },
                    Err(e) => match e.kind() {
                        std::io::ErrorKind::WouldBlock
                        | std::io::ErrorKind::TimedOut
                        | std::io::ErrorKind::Interrupted => {
                            info!("Error: {e}\r\n");
                            unsafe { vTaskDelay(10) };
                            continue;
                        }
                        _ => {
                            info!("Error: {e}\r\n");
                            continue;
                        }
                    },
                }
            }?;
            buffer.copy_within(data_len - leftover_len..data_len, 0);
            data_len = leftover_len;
            Ok::<_, anyhow::Error>(signal)
        }?;
        // let signal = IrSignal {
        //     packet: IrPacket {
        //         remote_type: RemoteType::Generic,
        //         receiver_id: 0x00FF,
        //         button: 0xB0,
        //     },
        //     repeat: None,
        // };
        // let signal = IrSignal {
        //     packet: IrPacket {
        //         remote_type: RemoteType::Generic,
        //         receiver_id: 0xA55A,
        //         button: 0x38,
        //     },
        //     repeat: Some(Repeat {
        //         times: 2,
        //         duration_between: Duration::from_secs_f64(0.027116677),
        //     }),
        // };
        println!("Sending signal: {signal:#?}");
        internal_led.set_low()?;
        for event in signal.encode() {
            match event.is_on {
                true => channel.enable()?,
                false => channel.disable()?,
            }
            spin_sleep::sleep(event.duration);
        }
        channel.disable()?;
        internal_led.set_high()?;
        println!("Sent signal");
    }
}
