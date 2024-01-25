use crate::{
    const_characteristics::create_const_characteristics,
    process_stdin::process_stdin,
    short_name::{get_short_name, NVS_TAG_SHORT_NAME},
    validate_short_name::validate_short_name,
};
use esp32_nimble::{
    enums::*, utilities::BleUuid, uuid128, BLEDevice, BLEReturnCode, NimbleProperties,
};
use esp_idf_hal::task;
use esp_idf_svc::nvs::{EspNvs, EspNvsPartition, NvsDefault};
use esp_idf_sys as _;
use futures::{channel::mpsc::channel, join, StreamExt};
use log::{info, warn};
use std::{
    rc::Rc,
    sync::{Arc, RwLock},
};

mod const_characteristics;
mod info;
mod process_stdin;
mod short_name;
mod stdin;
mod validate_short_name;

const INITIAL_PASSKEY: u32 = 123456;
const NVS_NAMESPACE: &str = "sensor_connect";
const NVS_TAG_PASSKEY: &str = "passkey";
const SERVICE_UUID: BleUuid = uuid128!("c5f93147-b051-4201-bb59-ff8f18db9876");
const SHORT_NAME_UUID: BleUuid = uuid128!("ec67e1ac-cdd0-44bd-9c03-aebc64968b68");
const PASSKEY_UUID: BleUuid = uuid128!("f0650e70-58ff-4b69-ab99-5d61c6db7e75");

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
    let name = get_short_name(&mut nvs);
    let passkey = {
        match nvs.get_u32(NVS_TAG_PASSKEY).unwrap() {
            Some(stored_passkey) => stored_passkey,
            None => {
                nvs.set_u32(NVS_TAG_PASSKEY, INITIAL_PASSKEY).unwrap();
                INITIAL_PASSKEY
            }
        }
    };
    info!("Passkey is: {:0>6}", passkey);

    let nvs = RwLock::new(nvs);

    let device = BLEDevice::take();
    device
        .security()
        .set_auth(AuthReq::all())
        .set_passkey(passkey)
        .set_io_cap(SecurityIOCap::DisplayOnly);

    let server = device.get_server();

    let (mut advertise_tx, mut advertise_rx) = channel::<()>(0);
    server.on_connect(move |server, desc| {
        ::log::info!("Client connected: {:?}", desc);

        if server.connected_count() < (esp_idf_sys::CONFIG_BT_NIMBLE_MAX_CONNECTIONS as _) {
            ::log::info!("Multi-connect support: start advertising");
            advertise_tx.try_send(()).unwrap();
        }
    });
    server.on_disconnect(|_desc, reason| {
        ::log::info!("Client disconnected ({:?})", BLEReturnCode(reason as _));
    });

    let ble_advertising = device.get_advertising();
    let device = Rc::new(RwLock::new(device));

    ble_advertising
        .name(name.as_str())
        .add_service_uuid(SERVICE_UUID);
    let ble_advertising = Arc::new(RwLock::new(ble_advertising));

    let service = server.create_service(SERVICE_UUID);

    create_const_characteristics(&service);

    let (mut short_name_tx, mut short_name_rx) = channel::<String>(0);
    let short_name_characteristic = service.lock().create_characteristic(
        SHORT_NAME_UUID,
        NimbleProperties::READ
            | NimbleProperties::WRITE
            | NimbleProperties::WRITE_ENC
            | NimbleProperties::WRITE_AUTHEN
            | NimbleProperties::NOTIFY,
    );
    let set_short_name = {
        let short_name_characteristic = short_name_characteristic.clone();
        let ble_advertising = ble_advertising.clone();
        let ref nvs = nvs;
        move |new_name: &str| {
            nvs.write()
                .unwrap()
                .set_str(NVS_TAG_SHORT_NAME, new_name)
                .unwrap();
            ble_advertising.write().unwrap().reset().unwrap();
            ble_advertising
                .write()
                .unwrap()
                .name(new_name)
                .add_service_uuid(SERVICE_UUID)
                .start()
                .unwrap();
            short_name_characteristic
                .lock()
                .set_value(new_name.as_bytes())
                .notify();
        }
    };
    short_name_characteristic
        .lock()
        .set_value(name.as_bytes())
        .on_write(
            move |args| match String::from_utf8(args.recv_data.to_vec()) {
                Ok(short_name) => match validate_short_name(&short_name) {
                    Ok(_) => set_short_name(&short_name),
                    Err(message) => warn!("{}", message),
                },
                Err(e) => {
                    args.reject();
                    warn!("Invalid short_name. Error: {:#?}", e);
                }
            },
        );

    let (mut passkey_tx, mut passkey_rx) = channel::<u32>(0);
    let passkey_characteristic = service.lock().create_characteristic(
        PASSKEY_UUID,
        NimbleProperties::READ
            | NimbleProperties::READ_ENC
            | NimbleProperties::READ_AUTHEN
            | NimbleProperties::WRITE
            | NimbleProperties::WRITE_ENC
            | NimbleProperties::WRITE_AUTHEN
            | NimbleProperties::NOTIFY,
    );
    passkey_characteristic
        .lock()
        .set_value(&passkey.to_be_bytes())
        .on_write(
            move |args| match <&[u8] as TryInto<[u8; 4]>>::try_into(args.recv_data) {
                Ok(new_passkey) => {
                    let new_passkey = u32::from_be_bytes(new_passkey);
                    passkey_tx.try_send(new_passkey).unwrap();
                }
                Err(e) => {
                    warn!(
                        "Pass key was not changed because it had an invalid length. Error: {:#?}",
                        e
                    );
                }
            },
        );
    let set_passkey = {
        |passkey| {
            device.write().unwrap().security().set_passkey(passkey);
            nvs.write()
                .unwrap()
                .set_u32(NVS_TAG_PASSKEY, passkey)
                .unwrap();
            passkey_characteristic
                .lock()
                .set_value(&passkey.to_be_bytes())
                .notify();
        }
    };

    ::log::info!(
        "bonded_addresses: {:?}",
        device.read().unwrap().bonded_addresses().unwrap()
    );

    join!(
        process_stdin(
            &short_name_characteristic,
            &set_short_name,
            &passkey_characteristic,
            &set_passkey
        ),
        async {
            while let Some(_) = advertise_rx.next().await {
                device.write().unwrap().get_advertising().start().unwrap();
            }
        },
        async {
            while let Some(short_name) = short_name_rx.next().await {
                set_short_name(&short_name);
            }
        },
        async {
            while let Some(passkey) = passkey_rx.next().await {
                set_passkey(passkey);
            }
        }
    );
}
