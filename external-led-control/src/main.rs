use esp_idf_hal::{
    gpio::{InterruptType, Level, PinDriver, Pull},
    peripherals::Peripherals,
    task::block_on,
};
use esp_idf_svc::nvs::{EspNvs, EspNvsPartition, NvsDefault};
use esp_idf_sys as _;
use futures::{
    prelude::*,
    stream::{select_all, unfold},
};
use std::{io::Error, pin::Pin, time::Duration};
use stdin::get_stdin_stream;

mod async_timeout;
mod stdin;
mod timer_stream;

fn main() {
    block_on(main_async());
}

async fn main_async() {
    // It is necessary to call this function once,
    // or else some patches to the runtime implemented by esp-idf-sys might not link properly.
    esp_idf_sys::link_patches();

    let nvs_default_partition = EspNvsPartition::<NvsDefault>::take().unwrap();

    let namespace = "led_namespace";
    let nvs = match EspNvs::new(nvs_default_partition, namespace, true) {
        Ok(nvs) => {
            println!("Got namespace {:?} from default partition", namespace);
            nvs
        }
        Err(e) => panic!("Could't get namespace {:?}", e),
    };
    let tag = "is_on";

    let peripherals = Peripherals::take().unwrap();
    let mut led = PinDriver::input_output(peripherals.pins.gpio8).unwrap();
    let mut button = PinDriver::input(peripherals.pins.gpio9).unwrap();
    button.set_pull(Pull::Down).unwrap();
    button.set_interrupt_type(InterruptType::PosEdge).unwrap();
    button.enable_interrupt().unwrap();

    fn on_to_level(on: bool) -> Level {
        match on {
            true => Level::Low,
            false => Level::High,
        }
    }

    let mut is_on;
    match nvs.get_u8(tag).unwrap() {
        Some(stored_is_on) => {
            is_on = stored_is_on == 1;
            println!("Got stored value for is_on: {}", is_on);
            led.set_level(on_to_level(is_on)).unwrap();
        }
        None => {
            println!("No stored value for is_on. Storing default value.");
            is_on = false;
            nvs.set_u8(tag, is_on.into()).unwrap();
            led.set_high().unwrap();
        }
    }

    #[derive(Debug)]
    enum Action {
        On,
        Off,
        Toggle,
    }

    let (line_stream, _stop_reading_stdin) = get_stdin_stream(Duration::from_millis(10));
    let stream = line_stream
        .map(|byte| Ok::<[u8; 1], Error>([byte]))
        .into_async_read()
        .lines()
        .filter_map(|line| async {
            if let Ok(line) = line {
                match line.as_str() {
                    "on" => Some(Action::On),
                    "off" => Some(Action::Off),
                    "toggle" => Some(Action::Toggle),
                    _ => None,
                }
            } else {
                None
            }
        });

    let button_stream = unfold(button, |mut button| async {
        button.wait_for_rising_edge().await.unwrap();
        Some((Action::Toggle, button))
    });

    let event_streams: Vec<Pin<Box<dyn Stream<Item = Action>>>> =
        vec![Box::pin(button_stream), Box::pin(stream)];
    let mut event_stream = select_all(event_streams);

    loop {
        let action_to_on = |action: Action| -> bool {
            match action {
                Action::On => true,
                Action::Off => false,
                Action::Toggle => !is_on,
            }
        };

        let action = event_stream.next().await.unwrap();
        is_on = action_to_on(action);
        led.set_level(on_to_level(is_on)).unwrap();
        println!("Storing new value for is_on: {}", is_on);
        nvs.set_u8(tag, is_on as u8).unwrap();
    }
}
