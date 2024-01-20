use esp32_nimble::{
    enums::*, utilities::BleUuid, uuid128, BLEDevice, BLEReturnCode, NimbleProperties,
};
use esp_idf_svc::nvs::{EspNvs, EspNvsPartition, NvsDefault};
use esp_idf_sys as _;
use random::Source;
use std::time::SystemTime;

const INITIAL_PASSKEY: u32 = 123456;
const RANDOM_BYTES: usize = 1;
const INITIAL_NAME: &str = "OpenSensor";
const NVS_NAMESPACE: &str = "sensor_connect";
const NVS_TAG_SHORT_NAME: &str = "short_name";
const SERVICE_UUID: BleUuid = uuid128!("c5f93147-b051-4201-bb59-ff8f18db9876");
const NAME_UUID: BleUuid = uuid128!("72e4028a-f727-4867-9ec4-25637a6eb834");
const VERSION_UUID: BleUuid = uuid128!("504fc887-3a39-4cd2-89f1-0fa6c9c55f22");
const HOMEPAGE_UUID: BleUuid = uuid128!("2f292fff-56e0-40b2-b8bd-cb1cc6937920");
const REPOSITORY_UUID: BleUuid = uuid128!("a2467465-8e29-436e-a0d4-6dd847193c89");
const AUTHORS_UUID: BleUuid = uuid128!("7ef914f3-9c94-45f9-ab77-26429fae3bc4");

// 31 bytes for advertising, minus 2 for idk, minus 16 for service uuid
const SHORT_NAME_MAX_LENGTH: usize = 31 - 2 - 16;

fn main() {
    // It is necessary to call this function once,
    // or else some patches to the runtime implemented by esp-idf-sys might not link properly.
    esp_idf_sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    let nvs_default_partition = EspNvsPartition::<NvsDefault>::take().unwrap();
    let mut nvs = EspNvs::new(nvs_default_partition, NVS_NAMESPACE, true).unwrap();
    let name = {
        // Add 1 cuz it needs an extra character for \0 (which we will trim)
        let mut buf = [0u8; SHORT_NAME_MAX_LENGTH + 1];
        let saved_name = nvs.get_str(NVS_TAG_SHORT_NAME, &mut buf).unwrap();
        match saved_name {
            Some(saved_name) => saved_name.trim_end_matches(char::from(0)).to_owned(),
            None => {
                let seed = SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos() as u64;
                let mut source = random::default(seed);
                let bytes =
                    hex::encode_upper(source.iter().take(RANDOM_BYTES).collect::<Vec<u8>>());
                let name = format!("{} {}", INITIAL_NAME, bytes);
                nvs.set_str(NVS_TAG_SHORT_NAME, name.as_str()).unwrap();
                name
            }
        }
    };

    let device = BLEDevice::take();
    device
        .security()
        .set_auth(AuthReq::all())
        .set_passkey(INITIAL_PASSKEY)
        .set_io_cap(SecurityIOCap::DisplayOnly);

    let server = device.get_server();
    server.on_connect(|_server, desc| {
        ::log::info!("Client connected: {:?}", desc);
    });
    server.on_disconnect(|_desc, reason| {
        ::log::info!("Client disconnected ({:?})", BLEReturnCode(reason as _));
    });

    let service = server.create_service(SERVICE_UUID);

    struct ConstCharacteristic {
        uuid: BleUuid,
        value: &'static str,
    }
    let const_characteristics = vec![
        ConstCharacteristic {
            uuid: NAME_UUID,
            value: env!("CARGO_PKG_NAME"),
        },
        ConstCharacteristic {
            uuid: VERSION_UUID,
            value: env!("CARGO_PKG_VERSION"),
        },
        ConstCharacteristic {
            uuid: HOMEPAGE_UUID,
            value: env!("CARGO_PKG_HOMEPAGE"),
        },
        ConstCharacteristic {
            uuid: REPOSITORY_UUID,
            value: env!("CARGO_PKG_REPOSITORY"),
        },
        ConstCharacteristic {
            uuid: AUTHORS_UUID,
            value: env!("CARGO_PKG_AUTHORS"),
        },
    ];
    for const_characteristic in const_characteristics {
        service
            .lock()
            .create_characteristic(const_characteristic.uuid, NimbleProperties::READ)
            .lock()
            .set_value(const_characteristic.value.as_bytes());
    }

    let ble_advertising = device.get_advertising();
    ble_advertising
        .name(name.as_str())
        .add_service_uuid(SERVICE_UUID)
        .start()
        .unwrap();

    ::log::info!("bonded_addresses: {:?}", device.bonded_addresses().unwrap());
}
