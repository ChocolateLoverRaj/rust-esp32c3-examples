use std::{
    borrow::BorrowMut,
    sync::{Arc, RwLock},
};

use esp32_nimble::{
    utilities::{mutex::Mutex, BleUuid},
    uuid128, BLECharacteristic, BLEDevice, BLEService, NimbleProperties,
};
use esp_idf_svc::nvs::{EspNvs, NvsDefault};
use futures::channel::mpsc::Sender;

use crate::get_short_name::get_short_name;

const NVS_TAG_BLE_ON: &str = "ble_on";
const DEFAULT_BLE_ON: bool = true;
const BLE_UUID_ON: BleUuid = uuid128!("3c534064-8559-45e8-84d1-761d1c5ef438");

#[derive(Clone)]
pub struct BleOnCharacteristic {
    characteristic: Arc<Mutex<BLECharacteristic>>,
    on_change_tx: Sender<()>,
    nvs: Arc<RwLock<EspNvs<NvsDefault>>>,
}

impl BleOnCharacteristic {
    pub fn get_initial_value(nvs: &mut EspNvs<NvsDefault>) -> bool {
        match nvs.get_i8(NVS_TAG_BLE_ON).unwrap() {
            Some(value) => value != 0,
            None => {
                nvs.set_i8(NVS_TAG_BLE_ON, DEFAULT_BLE_ON.into()).unwrap();
                DEFAULT_BLE_ON
            }
        }
    }

    fn encode(value: bool) -> [u8; 1] {
        [<bool as Into<u8>>::into(value)]
    }

    fn decode(value: &[u8]) -> bool {
        <&[u8] as TryInto<[u8; 1]>>::try_into(value).unwrap()[0] != 0
    }

    pub fn new(
        service: &Arc<Mutex<BLEService>>,
        nvs: &Arc<RwLock<EspNvs<NvsDefault>>>,
        on_change_tx: Sender<()>,
        initial_value: bool,
    ) -> Self {
        let characteristic = service.lock().create_characteristic(
            BLE_UUID_ON,
            NimbleProperties::READ
                | NimbleProperties::WRITE
                | NimbleProperties::WRITE_ENC
                | NimbleProperties::WRITE_AUTHEN
                | NimbleProperties::NOTIFY,
        );

        let ble_on_characteristic = Self {
            characteristic: characteristic.clone(),
            on_change_tx,
            nvs: nvs.clone(),
        };

        {
            let mut ble_on_characteristic = ble_on_characteristic.clone();
            characteristic
                .lock()
                .set_value(&Self::encode(initial_value))
                .on_write(move |args| {
                    let current_ble_on = Self::decode(args.current_data());
                    let new_ble_on = Self::decode(args.recv_data());
                    if new_ble_on != current_ble_on {
                        args.notify();
                        ble_on_characteristic.on_change_tx.try_send(()).unwrap();
                        ble_on_characteristic.save_and_start_or_stop(new_ble_on);
                    }
                });
        }

        ble_on_characteristic
    }

    pub fn get(&mut self) -> bool {
        Self::decode(self.characteristic.lock().value_mut().value())
    }

    fn save_and_start_or_stop(&mut self, on: bool) {
        self.nvs
            .write()
            .unwrap()
            .set_i8(NVS_TAG_BLE_ON, on.into())
            .unwrap();
        if on {
            BLEDevice::init();
            BLEDevice::take()
                .get_advertising()
                .lock()
                .name(&get_short_name(self.nvs.write().unwrap().borrow_mut()))
                .start()
                .unwrap();
        } else {
            BLEDevice::deinit();
        }
    }

    pub fn set_external(&mut self, on: bool) {
        if on != self.get() {
            self.on_change_tx.try_send(()).unwrap();
            self.save_and_start_or_stop(on);
            self.characteristic
                .lock()
                .set_value(&Self::encode(on))
                .notify();
        }
    }
}
