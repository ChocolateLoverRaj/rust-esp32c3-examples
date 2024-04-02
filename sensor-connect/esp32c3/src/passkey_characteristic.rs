use std::sync::{Arc, RwLock};

use esp32_nimble::{
    BLECharacteristic, BLEDevice, BLEService, NimbleProperties, OnWriteArgs, utilities::BleUuid,
    uuid128,
};
use esp_idf_svc::nvs::{EspNvs, NvsDefault};
use futures::channel::mpsc::Sender;
use log::warn;

use common::PASSKEY_UUID;

use crate::NVS_TAG_PASSKEY;

#[derive(Clone)]
pub struct PasskeyCharacteristic {
    characteristic: Arc<esp32_nimble::utilities::mutex::Mutex<BLECharacteristic>>,
    nvs: Arc<RwLock<EspNvs<NvsDefault>>>,
    on_change_sender: Sender<()>,
}

impl PasskeyCharacteristic {
    pub fn new(
        service: &Arc<esp32_nimble::utilities::mutex::Mutex<BLEService>>,
        initial_passkey: u32,
        nvs: Arc<RwLock<EspNvs<NvsDefault>>>,
        on_change_sender: Sender<()>,
    ) -> Self {
        let passkey_characteristic = service.lock().create_characteristic(
            BleUuid::from_uuid128_string(PASSKEY_UUID).unwrap(),
            NimbleProperties::READ
                | NimbleProperties::READ_ENC
                | NimbleProperties::READ_AUTHEN
                | NimbleProperties::WRITE
                | NimbleProperties::WRITE_ENC
                | NimbleProperties::WRITE_AUTHEN
                | NimbleProperties::NOTIFY,
        );

        let characteristic = Self {
            characteristic: passkey_characteristic.clone(),
            nvs,
            on_change_sender,
        };

        {
            let mut characteristic = characteristic.clone();
            passkey_characteristic
                .lock()
                .set_value(&initial_passkey.to_be_bytes())
                .on_write(
                    move |args| match <&[u8] as TryInto<[u8; 4]>>::try_into(args.recv_data()) {
                        Ok(new_passkey) => {
                            let new_passkey = u32::from_be_bytes(new_passkey);
                            characteristic.set_from_on_write(new_passkey, args);
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

        characteristic
    }

    pub fn get(&mut self) -> u32 {
        u32::from_be_bytes(
            <&[u8] as TryInto<[u8; 4]>>::try_into(self.characteristic.lock().value_mut().value())
                .unwrap(),
        )
    }

    fn set(&mut self, new_passkey: u32) {
        BLEDevice::take().security().set_passkey(new_passkey);
        self.nvs
            .write()
            .unwrap()
            .set_u32(NVS_TAG_PASSKEY, new_passkey)
            .unwrap();
    }

    fn set_from_on_write(&mut self, new_passkey: u32, on_write_args: &mut OnWriteArgs) {
        self.set(new_passkey);
        on_write_args.notify();
        self.on_change_sender.try_send(()).unwrap();
    }

    pub fn set_externally(&mut self, new_passkey: u32) {
        self.set(new_passkey);
        self.characteristic
            .lock()
            .set_value(&new_passkey.to_be_bytes())
            .notify();
        self.on_change_sender.try_send(()).unwrap();
    }
}
