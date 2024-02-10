use std::sync::{Arc, Mutex};

use esp32_nimble::{utilities::BleUuid, uuid128, BLEService, NimbleProperties, NimbleSub};
use futures::{
    channel::mpsc::{channel, UnboundedReceiver},
    Future, StreamExt,
};

use crate::vl53l0x_sensor::{DistanceData, DistanceSensor, DistanceSubscribable};

const BLE_DISTANCE: BleUuid = uuid128!("c85a22c0-ffa0-46f1-94c7-d108f8e4df9e");

pub fn create_distance_characteristic(
    service: &Arc<esp32_nimble::utilities::mutex::Mutex<BLEService>>,
    mut distance_subscribable: DistanceSubscribable,
    distance_sensor: Arc<futures::lock::Mutex<DistanceSensor>>,
) -> impl Future<Output = ()> {
    let characteristic = service.lock().create_characteristic(
        BLE_DISTANCE,
        NimbleProperties::READ | NimbleProperties::NOTIFY,
    );

    let subscribed_id = Mutex::new(None::<usize>);
    let (mut tx, mut rx) = channel::<UnboundedReceiver<DistanceData>>(0);

    characteristic
        .lock()
        .on_read({
            let distance_subscribable = distance_subscribable.clone();
            move |att_value, _| {
                if !distance_subscribable.is_subscribed() {
                    let distance = distance_sensor
                        .try_lock()
                        .unwrap()
                        .vl53l0x
                        .read_range_single_millimeters_blocking()
                        .unwrap();
                    log::info!("Distance: {}mm", distance);
                    att_value.set_value(&distance.to_be_bytes());
                }
            }
        })
        .on_subscribe({
            move |characteristic, _, sub| {
                let subscribed_count = characteristic.subscribed_count();
                if sub == NimbleSub::NOTIFY && subscribed_count == 1 {
                    let (receiver, id) = distance_subscribable.subscribe();
                    *subscribed_id.lock().unwrap() = Some(id);
                    tx.try_send(receiver).unwrap();
                } else if sub.is_empty() && subscribed_count == 0 {
                    distance_subscribable.unsubscribe(subscribed_id.lock().unwrap().unwrap());
                }
            }
        });

    async move {
        loop {
            let mut receiver = rx.next().await.unwrap();
            loop {
                match receiver.next().await {
                    Some(distance_data) => {
                        characteristic
                            .lock()
                            .set_value(&distance_data.to_bytes())
                            .notify();
                    }
                    None => break,
                };
            }
        }
    }
}
