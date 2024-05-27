#![feature(const_trait_impl, effects)]

use std::{
    borrow::BorrowMut,
    sync::{Arc, Mutex, RwLock},
};
use std::convert::Into;

use esp32_nimble::{
    BLEAdvertisementData, BLEDevice, BLEReturnCode, enums::*, utilities::BleUuid, uuid128,
};
use esp_idf_hal::{peripherals::Peripherals, task};
use esp_idf_svc::nvs::{EspNvs, EspNvsPartition, NvsDefault};
use esp_idf_sys as _;
use futures::{channel::mpsc::channel, join};
use futures::future::join3;
use log::info;

use common::{INITIAL_PASSKEY, SERVICE_UUID};

use crate::{
    ble_on_characteristic::BleOnCharacteristic,
    const_characteristics::create_const_characteristics,
    distance_characteristic::create_distance_characteristic,
    get_short_name::get_short_name,
    ir_characteristic::create_ir_characteristic,
    ir_sensor::{configure_and_get_ir_sensor, ir_loop},
    passkey_characteristic::PasskeyCharacteristic,
    process_stdin::{IrInput, process_stdin},
    short_name_characteristic::ShortNameCharacteristic,
    vl53l0x_sensor::{distance_loop, get_vl53l0x},
};
use crate::subscribable3::Subscribable3;

mod async_vl53l0x;
mod ble_on_characteristic;
mod const_characteristics;
mod distance_characteristic;
mod get_short_name;
mod info;
mod ir_characteristic;
mod ir_sensor;
mod passkey_characteristic;
mod process_stdin;
mod short_name_characteristic;
mod stdin;
mod subscribable2;
mod vl53l0x_sensor;
mod subscribable3;

const NVS_NAMESPACE: &str = "sensor_connect";
const NVS_TAG_PASSKEY: &str = "passkey";

fn main() {
    task::block_on(main_async());
}

async fn main_async() {
    // It is necessary to call this function once,
    // or else some patches to the runtime implemented by esp-idf-sys might not link properly.
    esp_idf_sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    let nvs_default_partition = EspNvsPartition::<NvsDefault>::take().unwrap();
    let mut nvs = EspNvs::new(nvs_default_partition, NVS_NAMESPACE, true).unwrap();
    let initial_name = get_short_name(&mut nvs);
    let initial_passkey = {
        match nvs.get_u32(NVS_TAG_PASSKEY).unwrap() {
            Some(stored_passkey) => stored_passkey,
            None => {
                nvs.set_u32(NVS_TAG_PASSKEY, INITIAL_PASSKEY).unwrap();
                INITIAL_PASSKEY
            }
        }
    };
    info!("Initial name: {:#?}", initial_name);
    info!("Passkey is: {:0>6}", initial_passkey);

    let nvs = Arc::new(RwLock::new(nvs));

    let device = BLEDevice::take();
    device
        .security()
        .set_auth(AuthReq::all())
        .set_passkey(initial_passkey)
        .set_io_cap(SecurityIOCap::DisplayOnly);

    let server = device.get_server();

    server.on_connect(move |server, desc| {
        ::log::info!("Client connected: {:?}", desc);

        if server.connected_count() < (esp_idf_sys::CONFIG_BT_NIMBLE_MAX_CONNECTIONS as _) {
            ::log::info!("Multi-connect support: start advertising");
            BLEDevice::take().get_advertising().lock().start().unwrap();
        }
    });
    server.on_disconnect(|_desc, reason| {
        ::log::info!("Client disconnected ({:?})", BLEReturnCode(reason as _));
    });

    let ble_advertising = device.get_advertising();

    ble_advertising
        .lock()
        .set_data(
            &mut BLEAdvertisementData::new()
                .name(initial_name.as_str())
                .add_service_uuid(BleUuid::from_uuid128_string(SERVICE_UUID).unwrap()),
        )
        .unwrap();

    let service = server.create_service(BleUuid::from_uuid128_string(SERVICE_UUID).unwrap());

    create_const_characteristics(&service);

    let (short_name_change_tx, short_name_change_rx) = channel::<()>(0);
    let mut short_name_characteristic =
        ShortNameCharacteristic::new(&service, &initial_name, nvs.clone(), short_name_change_tx);

    let (passkey_change_tx, passkey_change_rx) = channel::<()>(0);
    let mut passkey_characteristic =
        PasskeyCharacteristic::new(&service, initial_passkey, nvs.clone(), passkey_change_tx);

    let initial_ble_on = BleOnCharacteristic::get_initial_value(nvs.write().unwrap().borrow_mut());
    let (ble_on_change_tx, ble_on_change_rx) = channel::<()>(0);
    let mut ble_on_characteristic =
        BleOnCharacteristic::new(&service, &nvs.clone(), ble_on_change_tx, initial_ble_on);

    let peripherals = Peripherals::take().unwrap();

    let (ir_future, ir_subscribable) = {
        let ir_sensor =
            configure_and_get_ir_sensor(peripherals.pins.gpio10, peripherals.pins.gpio7)
                .map(|ir_sensor| Arc::new(Mutex::new(ir_sensor)));
        let ir = ir_sensor.map(|ir_sensor| {
            let (ir_future, ir_subscribable) = ir_loop(ir_sensor.clone(), peripherals.pins.gpio8);
            let ir_characteristic_loop =
                create_ir_characteristic(&service, ir_subscribable.clone(), ir_sensor.clone());
            (
                async {
                    join!(ir_future, ir_characteristic_loop);
                },
                (ir_subscribable.clone(), ir_sensor.clone()),
            )
        });
        if ir.is_some() {
            info!("IR sensor connected");
        } else {
            info!("IR sensor not connected");
        }
        match ir {
            Some((ir_future, stuff)) => (Some(ir_future), Some(stuff)),
            None => (None, None),
        }
    };

    let distance_sensor = get_vl53l0x(
        peripherals.pins.gpio2,
        peripherals.pins.gpio3,
        peripherals.i2c0,
        peripherals.pins.gpio1,
    )
        .ok()
        .map(|v| Arc::new(futures::lock::Mutex::new(v)));
    let maybe_distance = distance_sensor.clone().map(|distance_sensor| {
        let distance_subscribable = Arc::new(Subscribable3::default());
        let (distance_future, distance_rx) = distance_loop(distance_sensor.clone(), distance_subscribable.clone());
        (
            {
                let distance_sensor = distance_sensor.clone();
                // let distance_subscribable = distance_subscribable.clone();
                let distance_characteristic_future = create_distance_characteristic(
                    &service,
                    distance_sensor.clone(),
                    distance_rx.clone(),
                    distance_subscribable.clone()
                );
                async move {
                    join!(distance_future, distance_characteristic_future);
                }
            },
            distance_subscribable,
            distance_rx,
            distance_sensor
        )
    });
    if distance_sensor.is_some() {
        info!("Distance sensor connected");
    } else {
        info!("Distance sensor not connected");
    }
    let (distance_future, distance) = match maybe_distance {
        None => (None, None),
        Some((future, subscribable, rx, sensor)) => (Some(future), Some((subscribable, rx, sensor))),
    };

    ::log::info!(
        "bonded_addresses: {:?}",
        BLEDevice::take().bonded_addresses().unwrap()
    );

    if initial_ble_on {
        ble_advertising.lock().start().unwrap();
    } else {
        BLEDevice::deinit().unwrap();
    }

    join3(
        process_stdin(
            &mut short_name_characteristic,
            short_name_change_rx,
            &mut passkey_characteristic,
            passkey_change_rx,
            &mut ble_on_characteristic,
            ble_on_change_rx,
            ir_subscribable.map(|(ir_subscribable, ir_sensor)| IrInput {
                ir_sensor: ir_sensor.clone(),
                subscribable: ir_subscribable,
            }),
            distance,
        ),
        async {
            if let Some(future) = ir_future {
                future.await;
            }
        },
        async {
            if let Some(future) = distance_future {
                future.await;
            }
        }
    ).await;
}
