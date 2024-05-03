use std::mem;
use std::sync::{Arc, Mutex};
use std::time::SystemTime;
use async_channel::Receiver;

use esp32_nimble::{utilities::BleUuid, uuid128, BLEService, NimbleProperties, NimbleSub};
use futures::{channel::mpsc::{channel, UnboundedReceiver}, Future, join, StreamExt};
use futures::channel::mpsc::unbounded;
use futures::future::join;
use futures_signals::signal::{Mutable, SignalExt};
use log::info;
use common::distance_data::DistanceData;
use common::DISTANCE_UUID;
use crate::subscribable3::Subscribable3;

use crate::vl53l0x_sensor::{DistanceSensor};

pub fn create_distance_characteristic(
    service: &Arc<esp32_nimble::utilities::mutex::Mutex<BLEService>>,
    distance_sensor: Arc<futures::lock::Mutex<DistanceSensor>>,
    distance_rx: Receiver<DistanceData>,
    distance_subscribable: Arc<Subscribable3>
) -> impl Future<Output=()> {
    info!("Creating distance characteristic");
    let characteristic = service.lock().create_characteristic(
        BleUuid::from_uuid128_string(DISTANCE_UUID).unwrap(),
        NimbleProperties::READ | NimbleProperties::NOTIFY,
    );

    // let subscribed_id = Mutex::new(None::<usize>);
    // let (mut tx, mut rx) = channel::<UnboundedReceiver<DistanceData>>(0);
    let is_subscribed = Mutable::new(false);
    let mut subscription = None;

    characteristic
        .lock()
        .on_read({
            let is_subscribed = is_subscribed.clone();
            let distance_sensor = distance_sensor.clone();
            move |att_value, _| {
                if !is_subscribed.get() {
                    let distance = distance_sensor
                        .try_lock()
                        .unwrap()
                        .vl53l0x
                        .read_range_single_millimeters_blocking()
                        .unwrap();
                    info!("Reading distance without continuous: {}mm", distance);
                    att_value.set_value(&DistanceData {
                        distance,
                        time: SystemTime::now(),
                    }.to_bytes());
                }
            }
        })
        .on_subscribe({
            let is_subscribed = is_subscribed.clone();
            let distance_subscribable = distance_subscribable.clone();
            move |characteristic, _, sub| {
                let subscribed_count = characteristic.subscribed_count();
                if sub == NimbleSub::NOTIFY && subscribed_count == 1 {
                    subscription = Some(distance_subscribable.subscribe());
                    // is_subscribed.set(true);
                } else if sub.is_empty() && subscribed_count == 0 {
                    // is_subscribed.set(false);
                    mem::take(&mut subscription);
                }
            }
        });

    // async move {
    //     let mut stream = is_subscribed.signal().to_stream();
    //     loop {
    //         let yes = stream.next().await.unwrap();
    //         if yes {
    //             info!("Starting continuous distance measurement");
    //             distance_sensor.lock().await.vl53l0x.start_continuous(0).unwrap();
    //             loop {
    //                 let distance = distance_sensor.lock().await.read_range_mm_async().await.unwrap();
    //                 info!("(Continuously) got distance: {}mm", distance);
    //                 characteristic.lock().set_value(&DistanceData {
    //                     distance,
    //                     time: SystemTime::now(),
    //                 }.to_bytes()).notify();
    //                 if !is_subscribed.get() {
    //                     break;
    //                 }
    //             }
    //             info!("Stopping continuous distance measurement");
    //             distance_sensor.lock().await.vl53l0x.stop_continuous().unwrap()
    //         }
    //     }
    // }

    async move {
        loop {
            let distance_data = distance_rx.recv().await.unwrap();
            characteristic
                .lock()
                .set_value(&distance_data.to_bytes())
                .notify();
        }
    }
}
