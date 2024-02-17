use std::sync::{Arc, Mutex};

use wasm_bindgen_futures::spawn_local;
use wasm_bindgen_test::console_log;
use wasm_react::{callback, h, Callback, Component, VNode};
use web_sys::console;

#[derive(Clone, Copy)]
pub enum ConnectionType {
    Usb,
    Ble,
}

use crate::{
    ble_connection::BleConnection, connection::Connection, connection_enum::ConnectionEnum,
    usb_connection::UsbConnection,
};

#[derive(Clone)]
pub struct ConnectButton<T: Fn(ConnectionEnum) + Clone> {
    pub connection_type: ConnectionType,
    pub on_connect: T,
}

impl<T: Fn(ConnectionEnum) + Clone + 'static> Component for ConnectButton<T> {
    fn render(&self) -> VNode {
        h!(button)
            .disabled(!match self.connection_type {
                ConnectionType::Usb => UsbConnection::is_available(),
                ConnectionType::Ble => BleConnection::is_available(),
            })
            .on_click(&callback!({
                let connection_type = self.connection_type.clone();
                let on_connect = self.on_connect.clone();
                move |_| {
                    spawn_local(async move {
                        match match connection_type {
                            ConnectionType::Usb => UsbConnection::connect()
                                .await
                                .map(|usb_connection| ConnectionEnum::Usb(usb_connection)),
                            ConnectionType::Ble => BleConnection::connect()
                                .await
                                .map(|ble_connection| ConnectionEnum::Ble(ble_connection)),
                        } {
                            Ok(connection) => {
                                //
                                on_connect(connection)
                            }
                            Err(err) => {
                                console_log!("Error: {:?}", err);
                                console::log_1(&err);
                            }
                        };
                    })
                }
            }))
            .build("Connect")
    }
}
