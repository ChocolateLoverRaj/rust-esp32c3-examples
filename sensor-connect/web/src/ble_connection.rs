use std::{rc::Rc, time::Duration};

use common::SERVICE_UUID;
use futures::{lock::Mutex, stream::unfold, SinkExt, StreamExt};
use leptos::{create_effect, create_signal_from_stream, on_cleanup};
use try_again::{retry_async, Retry};
use wasm_bindgen::{closure::Closure, JsCast, JsValue};
use wasm_bindgen_futures::{js_sys::DataView, spawn_local, JsFuture};
use web_sys::{
    js_sys::{Array, JsString, Object, Uint8Array},
    window, BluetoothDevice, RequestDeviceOptions,
};

use crate::connection::{Connection, ConnectionBuilder};

use self::{
    get_service::get_service, get_short_name_characteristic::get_short_name_characteristic,
};

mod get_service;
mod get_short_name_characteristic;

#[derive(Debug)]
pub struct BleConnection {
    name_subscriber_count: Rc<Mutex<usize>>,
    device: BluetoothDevice,
}
impl Connection for BleConnection {
    fn get_connection_type(&self) -> String {
        "BLE".into()
    }

    fn get_name(&self) -> Box<dyn std::future::Future<Output = String> + Unpin> {
        let device = self.device.clone();
        Box::new(Box::pin(async move {
            let service = get_service(device).await;
            let characteristic = get_short_name_characteristic(service).await;
            let name: DataView = JsFuture::from(characteristic.read_value())
                .await
                .unwrap()
                .dyn_into()
                .unwrap();
            let name = String::from_utf8(Uint8Array::new(&name.buffer()).to_vec()).unwrap();
            name
        }))
    }

    fn watch_name(&self) -> leptos::prelude::ReadSignal<Option<String>> {
        let subscriber_count = self.name_subscriber_count.clone();
        let device = self.device.clone();
        let (tx, rx) = futures::channel::mpsc::unbounded::<()>();
        let tx = Rc::new(Mutex::new(tx));

        create_effect({
            let subscriber_count = subscriber_count.clone();
            let device = device.clone();
            move |_| {
                let subscriber_count = subscriber_count.clone();
                let device = device.clone();
                let tx = tx.clone();
                spawn_local(async move {
                    let mut subscriber_count = subscriber_count.lock().await;
                    let service = get_service(device).await;
                    let characteristic = get_short_name_characteristic(service).await;
                    characteristic
                        .add_event_listener_with_callback(
                            "characteristicvaluechanged",
                            Closure::wrap(Box::new(move || {
                                spawn_local({
                                    let tx = tx.clone();
                                    async move {
                                        tx.lock().await.send(()).await.unwrap();
                                    }
                                })
                            }) as Box<dyn Fn()>)
                            .into_js_value()
                            .dyn_ref()
                            .unwrap(),
                        )
                        .unwrap();
                    *subscriber_count += 1;
                    if *subscriber_count == 1 {
                        retry_async(
                            Retry {
                                max_tries: 10,
                                delay: Some(try_again::Delay::ExponentialBackoff {
                                    initial_delay: Duration::from_millis(10),
                                    max_delay: Some(Duration::from_secs(5)),
                                }),
                            },
                            window().unwrap(),
                            || async {
                                let result =
                                    JsFuture::from(characteristic.start_notifications()).await;
                                result
                            },
                        )
                        .await
                        .unwrap();
                    }
                })
            }
        });
        on_cleanup(move || {
            spawn_local(async move {
                let subscriber_count = subscriber_count.clone();
                let device = device.clone();
                let mut subscriber_count = subscriber_count.lock().await;
                *subscriber_count -= 1;
                if *subscriber_count == 0 {
                    let service = get_service(device).await;
                    let characteristic = get_short_name_characteristic(service).await;
                    JsFuture::from(characteristic.stop_notifications())
                        .await
                        .unwrap();
                }
            })
        });
        {
            let device = self.device.clone();
            create_signal_from_stream(Box::pin(
                unfold(Some(device.clone()), |first| async move {
                    match first {
                        Some(device) => Some((
                            async move {
                                let service = get_service(device).await;
                                let characteristic = get_short_name_characteristic(service).await;
                                let name: DataView = JsFuture::from(characteristic.read_value())
                                    .await
                                    .unwrap()
                                    .dyn_into()
                                    .unwrap();
                                let name =
                                    String::from_utf8(Uint8Array::new(&name.buffer()).to_vec())
                                        .unwrap();
                                name
                            }
                            .await,
                            None,
                        )),
                        None => None,
                    }
                })
                .chain(unfold(rx, move |mut rx| {
                    let device = device.clone();
                    async move {
                        let service = get_service(device).await;
                        let characteristic = get_short_name_characteristic(service).await;
                        rx.next().await.unwrap();
                        let name = String::from_utf8(
                            Uint8Array::new(&characteristic.value().unwrap().buffer()).to_vec(),
                        )
                        .unwrap();
                        Some((name, rx))
                    }
                })),
            ))
        }
    }

    fn set_name(&self, new_name: &str) -> Box<dyn futures::prelude::Future<Output = ()> + Unpin> {
        let device = self.device.clone();
        let mut name_bytes = new_name.as_bytes().to_owned();
        Box::new(Box::pin(async move {
            let service = get_service(device).await;
            let characteristic = get_short_name_characteristic(service).await;
            JsFuture::from(characteristic.write_value_with_u8_array(&mut name_bytes))
                .await
                .unwrap();
        }))
    }
}

#[derive(Debug)]
pub struct BleConnectionBuilder {}
impl ConnectionBuilder for BleConnectionBuilder {
    fn is_available() -> bool {
        window().unwrap().navigator().bluetooth().is_some()
    }

    async fn connect() -> Result<Box<dyn Connection>, JsValue> {
        // FIXME: Error handling
        let device: BluetoothDevice = JsFuture::from(
            window()
                .unwrap()
                .navigator()
                .bluetooth()
                .unwrap()
                .request_device(
                    &RequestDeviceOptions::new().filters(&Array::of1(
                        &Object::from_entries(&Array::of1(&Array::of2(
                            &JsString::from("services"),
                            &Array::of1(&JsString::from(SERVICE_UUID)),
                        )))
                        .unwrap(),
                    )),
                ),
        )
        .await?
        .dyn_into()?;
        JsFuture::from(device.gatt().unwrap().connect()).await?;
        Ok(Box::new(BleConnection {
            device,
            name_subscriber_count: Default::default(),
        }))
    }
}
