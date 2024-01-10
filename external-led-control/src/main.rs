use esp_idf_hal::{
    gpio::{Level, PinDriver},
    peripherals::Peripherals,
    task::block_on,
};
use esp_idf_svc::nvs::{EspNvs, EspNvsPartition, NvsDefault};
use esp_idf_sys as _;
use futures::{prelude::*, stream::select_all};
use log::info;
use std::{
    pin::Pin,
    sync::{Arc, RwLock},
};

use crate::{
    action::Action, ble_interface::BleInterface, button_interface::ButtonInterface,
    interface::Interface, stdin_interface::StdinInterface,
};

mod action;
mod async_timeout;
mod ble_interface;
mod button_interface;
mod interface;
mod stdin;
mod stdin_interface;
mod timer_stream;

fn main() {
    block_on(main_async());
}

async fn main_async() {
    // It is necessary to call this function once,
    // or else some patches to the runtime implemented by esp-idf-sys might not link properly.
    esp_idf_sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();
    esp_idf_svc::log::EspLogger
        .set_target_level("*", log::LevelFilter::Off)
        .unwrap();

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

    fn on_to_level(on: bool) -> Level {
        match on {
            true => Level::Low,
            false => Level::High,
        }
    }

    let is_on;
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
    let is_on = Arc::new(RwLock::new(is_on));

    let (button_interface, button_stream) = ButtonInterface::new(peripherals.pins.gpio9);
    let (stdin_interface, stdin_stream) = StdinInterface::new(is_on.clone());
    let (ble_interface, ble_stream) = BleInterface::new(is_on.clone());

    let mut interfaces: Vec<Box<dyn Interface>> = vec![
        Box::new(button_interface),
        Box::new(stdin_interface),
        Box::new(ble_interface),
    ];
    let event_streams: Vec<Pin<Box<dyn Stream<Item = Action>>>> = vec![
        Box::pin(button_stream),
        Box::pin(stdin_stream),
        Box::pin(ble_stream),
    ];
    let mut event_stream = select_all(event_streams);

    loop {
        let action_to_on = |action: Action| -> bool {
            match action {
                Action::On => true,
                Action::Off => false,
                Action::Toggle => !*is_on.as_ref().read().unwrap(),
            }
        };

        let action = event_stream.next().await.unwrap();
        *is_on.as_ref().write().unwrap() = action_to_on(action);
        let is_on = is_on.as_ref().read().unwrap();

        for interface in &mut interfaces {
            interface.notify_change()
        }

        led.set_level(on_to_level(*is_on)).unwrap();
        info!("Storing new value for is_on: {}", *is_on);
        nvs.set_u8(tag, *is_on as u8).unwrap();
    }
}
