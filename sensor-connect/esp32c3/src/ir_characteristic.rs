use std::sync::{Arc, Mutex};
use std::time::SystemTime;

use esp32_nimble::{utilities::BleUuid, uuid128, BLEService, NimbleProperties, NimbleSub};
use futures::{
    channel::mpsc::{channel, UnboundedReceiver},
    Future, StreamExt,
};
use log::info;
use common::BLE_IR;
use common::ir_data::IrData;

use crate::ir_sensor::{IrSensor, IrSubscribable};

pub fn create_ir_characteristic(
    service: &Arc<esp32_nimble::utilities::mutex::Mutex<BLEService>>,
    mut ir_subscribable: IrSubscribable,
    ir_sensor: Arc<Mutex<IrSensor>>,
) -> impl Future<Output = ()> {
    let characteristic = service
        .lock()
        .create_characteristic(BleUuid::from_uuid128_string(BLE_IR).unwrap(), NimbleProperties::READ | NimbleProperties::NOTIFY);

    let subscribed_id = Mutex::new(None::<usize>);
    let (mut tx, mut rx) = channel::<UnboundedReceiver<IrData>>(0);

    characteristic
        .lock()
        .on_read(move |att_value, _| {
            att_value.set_value(&IrData {
                is_receiving_light: ir_sensor
                    .lock()
                    .unwrap()
                    .turn_on_and_check_is_receiving_light()
                    .into(),
                time: SystemTime::now()
            }.to_bytes());
        })
        .on_subscribe({
            move |characteristic, _, sub| {
                let subscribed_count = characteristic.subscribed_count();
                if sub == NimbleSub::NOTIFY && subscribed_count == 1 {
                    info!("Start watching IR receiver");
                    let (receiver, id) = ir_subscribable.subscribe();
                    *subscribed_id.lock().unwrap() = Some(id);
                    tx.try_send(receiver).unwrap();
                } else if sub.is_empty() && subscribed_count == 0 {
                    info!("Stop watching IR receiver");
                    ir_subscribable.unsubscribe(subscribed_id.lock().unwrap().unwrap());
                    info!("unsubscribed from IR receiver");
                }
            }
        });

    async move {
        loop {
            let mut receiver = rx.next().await.unwrap();
            loop {
                match receiver.next().await {
                    Some(is_on) => {
                        info!("Received update! {:#?}", is_on);
                        characteristic.lock().set_value(&is_on.to_bytes()).notify();
                    }
                    None => break,
                };
            }
        }
    }
}
