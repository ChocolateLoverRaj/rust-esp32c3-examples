use std::{marker::PhantomData, rc::Rc, time::Duration};

use futures::{lock::Mutex, SinkExt, stream::unfold, StreamExt};
use leptos::{create_effect, create_signal_from_stream, on_cleanup};
use try_again::{Retry, retry_async};
use wasm_bindgen::{closure::Closure, JsCast};
use wasm_bindgen_futures::{JsFuture, spawn_local};
use wasm_bindgen_test::console_log;
use web_sys::{BluetoothRemoteGattCharacteristic, js_sys::DataView, window};

use crate::connection::Characteristic;

use super::ble_serializer::BleSerializer;

#[derive(Clone, Debug)]
pub struct BleCharacteristic<T, S: BleSerializer<T>> {
    subscriber_count: Rc<Mutex<usize>>,
    characteristic: BluetoothRemoteGattCharacteristic,
    _phantom_data_s: PhantomData<S>,
    _phantom_data_t: PhantomData<T>,
}

impl<T, S: BleSerializer<T>> Characteristic<T> for BleCharacteristic<T, S> {
    fn watch(&self) -> leptos::prelude::ReadSignal<Option<T>> {
        let subscriber_count = self.subscriber_count.clone();
        let characteristic = self.characteristic.clone();
        let (tx, rx) = futures::channel::mpsc::unbounded::<()>();
        let tx = Rc::new(Mutex::new(tx));

        console_log!("Watching name");

        create_effect({
            let subscriber_count = subscriber_count.clone();
            let characteristic = characteristic.clone();
            move |_| {
                let subscriber_count = subscriber_count.clone();
                let characteristic = characteristic.clone();
                let tx = tx.clone();
                spawn_local(async move {
                    let mut subscriber_count = subscriber_count.lock().await;
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
        on_cleanup({
            let characteristic = characteristic.clone();
            move || {
                spawn_local(async move {
                    let subscriber_count = subscriber_count.clone();
                    let characteristic = characteristic.clone();
                    let mut subscriber_count = subscriber_count.lock().await;
                    *subscriber_count -= 1;
                    if *subscriber_count == 0 {
                        JsFuture::from(characteristic.stop_notifications())
                            .await
                            .unwrap();
                    }
                })
            }
        });
        {
            let characteristic = characteristic.clone();
            create_signal_from_stream(Box::pin(
                unfold(Some(characteristic.clone()), |first| async move {
                    match first {
                        Some(characteristic) => Some((
                            async move {
                                let data: DataView = JsFuture::from(characteristic.read_value())
                                    .await
                                    .unwrap()
                                    .dyn_into()
                                    .unwrap();
                                S::deserialize(data)
                            }
                                .await,
                            None,
                        )),
                        None => None,
                    }
                })
                    .chain(unfold(rx, move |mut rx| {
                        let characteristic = characteristic.clone();
                        async move {
                            rx.next().await.unwrap();
                            let name = S::deserialize(characteristic.value().unwrap());
                            Some((name, rx))
                        }
                    })),
            ))
        }
    }

    fn set(&self, new_value: T) -> Box<dyn futures::prelude::Future<Output=()> + Unpin> {
        let characteristic = self.characteristic.clone();
        let mut data = S::serialize(new_value);
        Box::new(Box::pin(async move {
            JsFuture::from(characteristic.write_value_with_u8_array(&mut data))
                .await
                .unwrap();
        }))
    }
}

impl<T, S: BleSerializer<T>> BleCharacteristic<T, S> {
    pub fn new(characteristic: BluetoothRemoteGattCharacteristic) -> Self {
        Self {
            subscriber_count: Default::default(),
            characteristic,
            _phantom_data_s: Default::default(),
            _phantom_data_t: Default::default(),
        }
    }
}
