use esp32_nimble::{enums::*, utilities::BleUuid, BLEDevice, BLEReturnCode, NimbleProperties};
use esp_idf_sys as _;
use random::Source;
use std::time::SystemTime;

const INITIAL_PASSKEY: u32 = 123456;
const RANDOM_BYTES: usize = 1;
const INITIAL_NAME: &str = "Sensor Connect";

fn main() {
    // It is necessary to call this function once,
    // or else some patches to the runtime implemented by esp-idf-sys might not link properly.
    esp_idf_sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    let seed = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64;
    let mut source = random::default(seed);
    let bytes = hex::encode_upper(source.iter().take(RANDOM_BYTES).collect::<Vec<u8>>());
    let name = format!("{} {}", INITIAL_NAME, bytes);

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

    let service = server.create_service(BleUuid::Uuid16(0xABCD));

    let non_secure_characteristic = service
        .lock()
        .create_characteristic(BleUuid::Uuid16(0x1234), NimbleProperties::READ);
    non_secure_characteristic
        .lock()
        .set_value("non_secure_characteristic".as_bytes());

    let secure_characteristic = service.lock().create_characteristic(
        BleUuid::Uuid16(0x1235),
        NimbleProperties::READ | NimbleProperties::READ_ENC | NimbleProperties::READ_AUTHEN,
    );
    secure_characteristic
        .lock()
        .set_value("secure_characteristic".as_bytes());

    let ble_advertising = device.get_advertising();
    ble_advertising
        .name(name.as_str())
        .add_service_uuid(BleUuid::Uuid16(0xABCD))
        .start()
        .unwrap();

    ::log::info!("bonded_addresses: {:?}", device.bonded_addresses().unwrap());
}
