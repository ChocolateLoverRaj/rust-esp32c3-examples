use futures::{SinkExt, StreamExt};
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::{
    BluetoothDevice,
    js_sys::{Array, JsString, Object}, RequestDeviceOptions, window,
};

use common::{SERVICE_UUID, SHORT_NAME_UUID};

use crate::ble_connection::ble_string_serializer::BleStringSerializer;
use crate::ble_connection::ble_u32_serializer::BleU32Serializer;
use crate::ble_connection::get_characteristic::get_characteristic;
use crate::connection::{Characteristic, Connection, ConnectionBuilder};

use self::{
    ble_characteristic::BleCharacteristic, get_service::get_service,
    get_short_name_characteristic::get_short_name_characteristic,
};

mod ble_characteristic;
mod ble_serializer;
mod get_service;
mod get_short_name_characteristic;
mod ble_string_serializer;
mod ble_u32_serializer;
mod get_characteristic;

#[derive(Debug)]
pub struct BleConnection {
    name_characteristic: BleCharacteristic<String, BleStringSerializer>,
    passkey_characteristic: BleCharacteristic<u32, BleU32Serializer>,
}

impl Connection for BleConnection {
    fn get_connection_type(&self) -> String {
        "BLE".into()
    }

    fn name(&self) -> Box<dyn crate::connection::Characteristic<String>> {
        Box::new(self.name_characteristic.clone())
    }

    fn passkey(&self) -> Box<dyn Characteristic<u32>> {
        Box::new(self.passkey_characteristic.clone())
    }
}

#[derive(Debug)]
pub struct BleConnectionBuilder {}

impl ConnectionBuilder for BleConnectionBuilder {
    fn is_available() -> bool {
        window().unwrap().navigator().bluetooth().is_some()
    }

    async fn connect() -> Result<Box<dyn Connection>, JsValue> {
        let device: BluetoothDevice = JsFuture::from(
            window()
                .unwrap()
                .navigator()
                .bluetooth()
                .unwrap()
                .request_device(
                    &RequestDeviceOptions::new().filters(&Array::of1(
                        &Object::from_entries(&Array::of1(&Array::of2(
                            &JsString::from("services"),
                            &Array::of1(&JsString::from(SERVICE_UUID)),
                        )))
                            .unwrap(),
                    )),
                ),
        )
            .await?
            .dyn_into()?;
        JsFuture::from(device.gatt().unwrap().connect()).await?;
        let service = get_service(&device).await;
        let characteristic = get_short_name_characteristic(&service).await;
        Ok(Box::new(BleConnection {
            name_characteristic: BleCharacteristic::new(characteristic),
            passkey_characteristic: BleCharacteristic::new(get_characteristic(&service, SHORT_NAME_UUID).await),
        }))
    }
}
