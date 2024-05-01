use common::{BLE_IR, BLE_ON_UUID, DISTANCE_UUID, PASSKEY_UUID, SERVICE_UUID, SHORT_NAME_UUID};
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use wasm_bindgen_test::console_log;
use web_sys::{
    BluetoothDevice,
    js_sys::{Array, JsString, Object}, RequestDeviceOptions, window,
};
use common::distance_data::DistanceData;
use common::ir_data::IrData;
use crate::ble_connection::ble_bool_serializer::BleBoolSerializer;

use crate::ble_connection::ble_string_serializer::BleStringSerializer;
use crate::ble_connection::ble_u32_serializer::BleU32Serializer;
use crate::ble_connection::distance_data_serializer::DistanceDataSerializer;
use crate::ble_connection::get_characteristic::get_characteristic;
use crate::ble_connection::ir_data_serializer::IrDataSerializer;
use crate::connection::{Characteristic, Connection, ConnectionBuilder};

use self::{
    ble_characteristic::BleCharacteristic, get_service::get_service,
};

mod ble_characteristic;
mod ble_serializer;
mod ble_string_serializer;
mod ble_u32_serializer;
mod get_characteristic;
mod get_service;
mod ble_bool_serializer;
mod ir_data_serializer;
mod distance_data_serializer;

#[derive(Debug)]
pub struct BleConnection {
    name_characteristic: BleCharacteristic<String, BleStringSerializer>,
    passkey_characteristic: BleCharacteristic<u32, BleU32Serializer>,
    ble_on_characteristic: BleCharacteristic<bool, BleBoolSerializer>,
    ir_characteristic: Option<BleCharacteristic<IrData, IrDataSerializer>>,
    distance_characteristic: Option<BleCharacteristic<DistanceData, DistanceDataSerializer>>,
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

    fn ble_on(&self) -> Box<dyn Characteristic<bool>> {
        Box::new(self.ble_on_characteristic.clone())
    }

    fn get_ir_led_characteristic(&self) -> Option<Box<dyn Characteristic<IrData>>> {
        self.ir_characteristic.clone().map(|characteristic| {
            let r: Box<dyn Characteristic<IrData>> = Box::new(characteristic);
            r
        })
    }

    fn get_distance_characteristic(&self) -> Option<Box<dyn Characteristic<DistanceData>>> {
        console_log!("Distance characteristic: {:#?}", self.distance_characteristic);
        self.distance_characteristic.clone().map(|characteristic| {
            let r: Box<dyn Characteristic<DistanceData>> = Box::new(characteristic);
            r
        })
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
        Ok(Box::new(BleConnection {
            name_characteristic: BleCharacteristic::new(get_characteristic(&service, SHORT_NAME_UUID).await.unwrap()),
            passkey_characteristic: BleCharacteristic::new(
                get_characteristic(&service, PASSKEY_UUID).await.unwrap(),
            ),
            ble_on_characteristic: BleCharacteristic::new(get_characteristic(&service, BLE_ON_UUID).await.unwrap()),
            ir_characteristic: get_characteristic(&service, BLE_IR).await.map(|characteristic| BleCharacteristic::new(characteristic)),
            distance_characteristic: get_characteristic(&service, DISTANCE_UUID).await.map(|characteristic| BleCharacteristic::new(characteristic)),
        }))
    }
}
