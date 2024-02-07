use crate::{
    ble_on_characteristic::BleOnCharacteristic,
    const_characteristics::create_const_characteristics,
    get_short_name::get_short_name,
    ir_characteristic::create_ir_characteristic,
    ir_sensor::{configure_and_get_receiver_pin, ir_loop},
    passkey_characteristic::PasskeyCharacteristic,
    process_stdin::process_stdin,
    short_name_characteristic::ShortNameCharacteristic,
};
use esp32_nimble::{enums::*, utilities::BleUuid, uuid128, BLEDevice, BLEReturnCode};
use esp_idf_hal::{peripherals::Peripherals, task};
use esp_idf_svc::nvs::{EspNvs, EspNvsPartition, NvsDefault};
use esp_idf_sys as _;
use futures::{channel::mpsc::channel, join};
use log::info;
use std::{
    borrow::BorrowMut,
    sync::{Arc, Mutex, RwLock},
};

mod ble_on_characteristic;
mod const_characteristics;
mod get_short_name;
mod info;
mod ir_characteristic;
mod ir_sensor;
mod passkey_characteristic;
mod process_stdin;
mod short_name_characteristic;
mod stdin;
mod subscribable2;
mod validate_short_name;

const INITIAL_PASSKEY: u32 = 123456;
const NVS_NAMESPACE: &str = "sensor_connect";
const NVS_TAG_PASSKEY: &str = "passkey";
const SERVICE_UUID: BleUuid = uuid128!("c5f93147-b051-4201-bb59-ff8f18db9876");

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
        .name(initial_name.as_str())
        .add_service_uuid(SERVICE_UUID);

    let service = server.create_service(SERVICE_UUID);

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
    let receiver_pin = Arc::new(Mutex::new(configure_and_get_receiver_pin(
        peripherals.pins.gpio5,
    )));
    let (ir_future, ir_subscribable) = ir_loop(receiver_pin.clone(), peripherals.pins.gpio8);

    let ir_characteristic_loop =
        create_ir_characteristic(&service, ir_subscribable.clone(), receiver_pin.clone());

    ::log::info!(
        "bonded_addresses: {:?}",
        BLEDevice::take().bonded_addresses().unwrap()
    );

    if initial_ble_on {
        ble_advertising.lock().start().unwrap();
    } else {
        BLEDevice::deinit();
    }

    join!(
        process_stdin(
            &mut short_name_characteristic,
            short_name_change_rx,
            &mut passkey_characteristic,
            passkey_change_rx,
            &mut ble_on_characteristic,
            ble_on_change_rx,
            ir_subscribable.clone(),
            receiver_pin.clone(),
        ),
        ir_future,
        ir_characteristic_loop
    );
}
