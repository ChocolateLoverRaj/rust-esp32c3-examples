use std::sync::{Arc, RwLock};

use crate::{action::Action, interface::Interface};
use esp32_nimble::{
    utilities::mutex::Mutex, uuid128, BLECharacteristic, BLEDevice, NimbleProperties,
};
use futures::{channel::mpsc::channel, executor::block_on, prelude::*, stream::unfold, Stream};

pub struct BleInterface {
    value: Arc<RwLock<bool>>,
    characteristic: Arc<Mutex<BLECharacteristic>>,
}

impl BleInterface {
    fn convert_value(value: &Arc<RwLock<bool>>) -> [u8; 1] {
        [*value.read().unwrap() as u8]
    }

    fn get_value(&self) -> [u8; 1] {
        Self::convert_value(&self.value)
    }

    pub fn new(value: Arc<RwLock<bool>>) -> (Self, impl Stream<Item = Action>) {
        let ble_device = BLEDevice::take();

        let server = ble_device.get_server();
        server.on_connect(|server, desc| {
            server
                .update_conn_params(desc.conn_handle(), 24, 48, 0, 60)
                .unwrap();

            ble_device.get_advertising().start().unwrap();
        });
        let service_uuid = uuid128!("a78b10d1-0ef3-454f-8ddd-b09fb08fdbe6");
        let service = server.create_service(service_uuid);

        // A writable characteristic.
        let characteristic = service.lock().create_characteristic(
            uuid128!("f935f617-1ebf-4b72-9ce8-732c2d7f531c"),
            NimbleProperties::READ | NimbleProperties::WRITE | NimbleProperties::NOTIFY,
        );
        let (mut tx, rx) = channel::<Action>(1);
        characteristic
            .lock()
            .set_value(&Self::convert_value(&value))
            .on_write(move |args| {
                block_on(async {
                    let parse_action = || -> Option<Action> {
                        Some(std::str::from_utf8(args.recv_data).ok()?.parse().ok()?)
                    };
                    if let Some(action) = parse_action() {
                        tx.send(action).await.unwrap();
                    }
                });
            });

        let ble_advertising = ble_device.get_advertising();
        ble_advertising
            .name("ESP32-C3 Built-in LED")
            .add_service_uuid(service_uuid);
        ble_advertising.start().unwrap();

        let stream = unfold(rx, |mut rx| async {
            let action = rx.next().await.unwrap();
            Some((action, rx))
        });

        let ble_interface = Self {
            value,
            characteristic,
        };

        (ble_interface, stream)
    }
}

impl Interface for BleInterface {
    fn notify_change(&mut self) {
        self.characteristic
            .lock()
            .set_value(&self.get_value())
            .notify();
    }

    fn stop(self) {
        todo!()
    }
}
