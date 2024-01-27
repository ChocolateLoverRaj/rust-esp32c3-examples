use crate::{
    const_characteristics::create_const_characteristics, get_short_name::get_short_name,
    process_stdin::process_stdin, short_name_characteristic::ShortNameCharacteristic,
};
use esp32_nimble::{
    enums::*, utilities::BleUuid, uuid128, BLEDevice, BLEReturnCode, NimbleProperties,
};
use esp_idf_hal::task;
use esp_idf_svc::nvs::{EspNvs, EspNvsPartition, NvsDefault};
use esp_idf_sys as _;
use log::{info, warn};
use std::sync::{Arc, Mutex, RwLock};

mod const_characteristics;
mod get_short_name;
mod info;
mod process_stdin;
mod short_name_characteristic;
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

    let nvs = Arc::new(RwLock::new(nvs));

    let device = BLEDevice::take();
    device
        .security()
        .set_auth(AuthReq::all())
        .set_passkey(passkey)
        .set_io_cap(SecurityIOCap::DisplayOnly);

    let server = device.get_server();

    server.on_connect(move |server, desc| {
        ::log::info!("Client connected: {:?}", desc);

        if server.connected_count() < (esp_idf_sys::CONFIG_BT_NIMBLE_MAX_CONNECTIONS as _) {
            ::log::info!("Multi-connect support: start advertising");
            BLEDevice::take().get_advertising().start().unwrap();
        }
    });
    server.on_disconnect(|_desc, reason| {
        ::log::info!("Client disconnected ({:?})", BLEReturnCode(reason as _));
    });

    let ble_advertising = device.get_advertising();

    ble_advertising
        .name(name.as_str())
        .add_service_uuid(SERVICE_UUID);

    let service = server.create_service(SERVICE_UUID);

    create_const_characteristics(&service);

    let mut short_name_characteristic = ShortNameCharacteristic::new(&service, &name, nvs.clone());
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
    let set_passkey = Arc::new(Mutex::new({
        let nvs = nvs.clone();
        let passkey_characteristic = passkey_characteristic.clone();

        {
            move |passkey| {
                BLEDevice::take().security().set_passkey(passkey);
                nvs.write()
                    .unwrap()
                    .set_u32(NVS_TAG_PASSKEY, passkey)
                    .unwrap();
                passkey_characteristic
                    .lock()
                    .set_value(&passkey.to_be_bytes())
                    .notify();
            }
        }
    }));
    {
        let set_passkey = set_passkey.clone();

        passkey_characteristic
            .lock()
            .set_value(&passkey.to_be_bytes())
            .on_write(
                move |args| match <&[u8] as TryInto<[u8; 4]>>::try_into(args.recv_data) {
                    Ok(new_passkey) => {
                        let new_passkey = u32::from_be_bytes(new_passkey);
                        set_passkey.lock().unwrap()(new_passkey);
                    }
                    Err(e) => {
                        warn!(
                        "Pass key was not changed because it had an invalid length. Error: {:#?}",
                        e
                    );
                    }
                },
            );
    }

    ::log::info!(
        "bonded_addresses: {:?}",
        BLEDevice::take().bonded_addresses().unwrap()
    );

    ble_advertising.start().unwrap();

    process_stdin(
        &mut short_name_characteristic,
        &passkey_characteristic,
        &set_passkey,
    )
    .await;
}
