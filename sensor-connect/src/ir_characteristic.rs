use std::sync::{Arc, Mutex};

use esp32_nimble::{utilities::BleUuid, uuid128, BLEService, NimbleProperties, NimbleSub};
use esp_idf_hal::gpio::{AnyIOPin, Input, PinDriver};
use futures::{
    channel::mpsc::{channel, UnboundedReceiver},
    Future, StreamExt,
};
use log::info;

use crate::ir_sensor::{is_receiving_light, IrData, IrSubscribable};

const BLE_IR: BleUuid = uuid128!("51b80f42-a10e-4912-852b-b155a5610557");

pub fn create_ir_characteristic(
    service: &Arc<esp32_nimble::utilities::mutex::Mutex<BLEService>>,
    mut ir_subscribable: IrSubscribable,
    receiver_pin: Arc<Mutex<PinDriver<'static, AnyIOPin, Input>>>,
) -> impl Future<Output = ()> {
    let characteristic = service
        .lock()
        .create_characteristic(BLE_IR, NimbleProperties::READ | NimbleProperties::NOTIFY);

    let subscribed_id = Mutex::new(None::<usize>);
    let (mut tx, mut rx) = channel::<UnboundedReceiver<IrData>>(0);

    characteristic
        .lock()
        .on_read(move |att_value, _| {
            att_value.set_value(&[is_receiving_light(&mut receiver_pin.lock().unwrap()).into()])
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
