use std::sync::{Arc, RwLock};

use esp32_nimble::{
    BLEAdvertisementData,
    BLECharacteristic, BLEDevice, BLEService, NimbleProperties, OnWriteArgs, utilities::{BleUuid, mutex::Mutex},
    uuid128,
};
use esp_idf_svc::nvs::{EspNvs, NvsDefault};
use futures::channel::mpsc::Sender;
use log::warn;

use common::SHORT_NAME_UUID;
use common::validate_short_name::validate_short_name;

use crate::{get_short_name::NVS_TAG_SHORT_NAME, SERVICE_UUID};

#[derive(Clone)]
pub struct ShortNameCharacteristic {
    characteristic: Arc<Mutex<BLECharacteristic>>,
    nvs: Arc<RwLock<EspNvs<NvsDefault>>>,
    on_change_sender: Sender<()>,
}

impl ShortNameCharacteristic {
    pub fn new(
        service: &Arc<Mutex<BLEService>>,
        initial_short_name: &str,
        nvs: Arc<RwLock<EspNvs<NvsDefault>>>,
        on_change_sender: Sender<()>,
    ) -> ShortNameCharacteristic {
        let short_name_characteristic = service.lock().create_characteristic(
            BleUuid::from_uuid128_string(SHORT_NAME_UUID).unwrap(),
            NimbleProperties::READ
                | NimbleProperties::WRITE
                | NimbleProperties::WRITE_ENC
                | NimbleProperties::WRITE_AUTHEN
                | NimbleProperties::NOTIFY,
        );

        let characteristic = Self {
            characteristic: short_name_characteristic.clone(),
            nvs,
            on_change_sender,
        };

        {
            let mut characteristic = characteristic.clone();
            short_name_characteristic
                .lock()
                .set_value(initial_short_name.as_bytes())
                .on_write(
                    move |args| match String::from_utf8(args.recv_data().to_vec()) {
                        Ok(short_name) => match validate_short_name(&short_name) {
                            Ok(_) => {
                                characteristic.set_in_on_write(&short_name, args);
                            }
                            Err(message) => warn!("{}", message),
                        },
                        Err(e) => {
                            args.reject();
                            warn!("Invalid short_name. Error: {:#?}", e);
                        }
                    },
                );
        }

        characteristic
    }

    pub fn get(&mut self) -> String {
        String::from_utf8(self.characteristic.lock().value_mut().value().to_vec()).unwrap()
    }

    // Doesn't call notify or change value
    fn set(&mut self, new_name: &str) {
        self.nvs
            .write()
            .unwrap()
            .set_str(NVS_TAG_SHORT_NAME, new_name)
            .unwrap();
        let mut ble_advertising = BLEDevice::take().get_advertising().lock();
        ble_advertising.reset().unwrap();
        ble_advertising
            .set_data(
                &mut BLEAdvertisementData::new()
                    .name(new_name)
                    .add_service_uuid(BleUuid::from_uuid128_string(SERVICE_UUID).unwrap()),
            )
            .unwrap();
        ble_advertising.start().unwrap()
    }

    fn set_in_on_write(&mut self, new_name: &str, on_write_args: &mut OnWriteArgs) {
        self.set(new_name);
        on_write_args.notify();
        self.on_change_sender.try_send(()).unwrap();
    }

    pub fn set_externally(&mut self, new_name: &str) {
        self.set(new_name);
        self.characteristic
            .lock()
            .set_value(new_name.as_bytes())
            .notify();
        self.on_change_sender.try_send(()).unwrap();
    }
}
