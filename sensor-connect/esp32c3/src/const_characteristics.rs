use esp32_nimble::{
    utilities::{mutex::Mutex, BleUuid},
    uuid128, BLEService, NimbleProperties,
};

use crate::info::get_info;

const PACKAGE_NAME_UUID: BleUuid = uuid128!("72e4028a-f727-4867-9ec4-25637a6eb834");
const VERSION_UUID: BleUuid = uuid128!("504fc887-3a39-4cd2-89f1-0fa6c9c55f22");
const HOMEPAGE_UUID: BleUuid = uuid128!("2f292fff-56e0-40b2-b8bd-cb1cc6937920");
const REPOSITORY_UUID: BleUuid = uuid128!("a2467465-8e29-436e-a0d4-6dd847193c89");
const AUTHORS_UUID: BleUuid = uuid128!("7ef914f3-9c94-45f9-ab77-26429fae3bc4");

pub fn create_const_characteristics(service: &Mutex<BLEService>) {
    struct ConstCharacteristic {
        uuid: BleUuid,
        value: String,
    }
    let info = get_info();
    let const_characteristics = vec![
        ConstCharacteristic {
            uuid: PACKAGE_NAME_UUID,
            value: info.name,
        },
        ConstCharacteristic {
            uuid: VERSION_UUID,
            value: info.version,
        },
        ConstCharacteristic {
            uuid: HOMEPAGE_UUID,
            value: info.homepage,
        },
        ConstCharacteristic {
            uuid: REPOSITORY_UUID,
            value: info.repository,
        },
        ConstCharacteristic {
            uuid: AUTHORS_UUID,
            value: info.authors,
        },
    ];
    for const_characteristic in const_characteristics {
        service
            .lock()
            .create_characteristic(const_characteristic.uuid, NimbleProperties::READ)
            .lock()
            .set_value(const_characteristic.value.as_bytes());
    }
}
