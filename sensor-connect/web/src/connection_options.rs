use std::sync::{Arc, Mutex};

use wasm_react::{h, Callback, Component, VNode};

use crate::{
    connect_button::{ConnectButton, ConnectionType},
    connection_enum::ConnectionEnum,
};

pub struct ConnectionOptions<T: Fn(ConnectionEnum) + Clone> {
    pub on_connect: T,
}

impl<T: Fn(ConnectionEnum) + Clone + 'static> Component for ConnectionOptions<T> {
    fn render(&self) -> VNode {
        h!(div).build((
            h!(h1).build("SensorConnect"),
            h!(p).build("Connect to an ESP32-C3 Super-Mini to read and graph sensor data"),
            h!(h1).build("Connection options"),
            h!(table).build((h!(tbody).build((
                h!(tr).build((
                    h!(th).build("Connection type"),
                    h!(td).build("USB"),
                    h!(td).build("BLE (Bluetooth Low Energy)"),
                )),
                h!(tr).build((
                    h!(th).build("Chromebook"),
                    h!(td).build("Yes (Chromium)"),
                    h!(td).build("Yes (Chromium)"),
                )),
                h!(tr).build((
                    h!(th).build("Android"),
                    h!(td).build("No"),
                    h!(td).build("Yes (Chromium)"),
                )),
                h!(tr).build((
                    h!(th).build("iPhone"),
                    h!(td).build("No"),
                    h!(td).build((
                        "Yes (",
                        h!(a)
                            .href(
                                "https://apps.apple.com/us/app/bluefy-web-ble-browser/id1492822055",
                            )
                            .build("Blueify"),
                        ")",
                    )),
                )),
                h!(tr).build((
                    h!(th).build("MacBook"),
                    h!(td).build("Yes (but not tested)"),
                    h!(td).build("Yes (but not tested)"),
                )),
                h!(tr).build((
                    h!(th).build("Windows"),
                    h!(td).build("Yes (Chromium)"),
                    h!(td).build("Yes (Chromium)"),
                )),
                h!(tr).build((
                    h!(th).build("Linux Desktop"),
                    h!(td).build("Yes (Chromium)"),
                    h!(td).build("Yes (Chromium)"),
                )),
                h!(tr).build((
                    h!(th).build("Maximum Computers"),
                    h!(td).build("1 (because you can't share a USB port with multiple computers)"),
                    h!(td).build("9 (in theory, tested with 4)"),
                )),
                h!(tr).build((
                    h!(th).build("Power Usage"),
                    h!(td).build("Low (my guess is 0.1W"),
                    h!(td).build("Medium (my guess is 1W)"),
                )),
                h!(tr).build((
                    h!(th).build(()),
                    h!(td).build(
                        ConnectButton {
                            connection_type: ConnectionType::Usb,
                            on_connect: self.on_connect.clone(),
                        }
                        .build(),
                    ),
                    h!(td).build(
                        ConnectButton {
                            connection_type: ConnectionType::Usb,
                            on_connect: self.on_connect.clone(),
                        }
                        .build(),
                    ),
                )),
            )),)),
            h!(h1).build("About"),
            h!(a).href(env!("CARGO_PKG_REPOSITORY")).build("GitHub"),
        ))
    }
}
