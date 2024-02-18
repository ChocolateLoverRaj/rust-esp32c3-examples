use common::{SERVICE_UUID, SHORT_NAME_UUID};
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::{js_sys::DataView, JsFuture};
use web_sys::{
    js_sys::{Array, JsString, Object},
    window, BluetoothDevice, BluetoothRemoteGattCharacteristic, BluetoothRemoteGattService,
    RequestDeviceOptions, TextDecoder,
};

use crate::connection::{Connection, ConnectionBuilder};

#[derive(Debug)]
pub struct BleConnection {
    device: BluetoothDevice,
}
impl Connection for BleConnection {
    fn get_connection_type(&self) -> String {
        "BLE".into()
    }

    fn get_name<'a>(&'a self) -> Box<dyn std::future::Future<Output = String> + Unpin + 'a> {
        Box::new(Box::pin(async {
            let service: BluetoothRemoteGattService = JsFuture::from(
                self.device
                    .gatt()
                    .unwrap()
                    .get_primary_service_with_str(SERVICE_UUID),
            )
            .await
            .unwrap()
            .dyn_into()
            .unwrap();
            let characteristic: BluetoothRemoteGattCharacteristic =
                JsFuture::from(service.get_characteristic_with_str(SHORT_NAME_UUID))
                    .await
                    .unwrap()
                    .dyn_into()
                    .unwrap();
            let name: DataView = JsFuture::from(characteristic.read_value())
                .await
                .unwrap()
                .dyn_into()
                .unwrap();
            let name = TextDecoder::new()
                .unwrap()
                .decode_with_buffer_source(&name.dyn_into().unwrap())
                .unwrap();
            name
        }))
    }
}

#[derive(Debug)]
pub struct BleConnectionBuilder {}
impl ConnectionBuilder for BleConnectionBuilder {
    fn is_available() -> bool {
        window().unwrap().navigator().bluetooth().is_some()
    }

    async fn connect() -> Result<Box<dyn Connection>, JsValue> {
        // FIXME: Error handling
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
        Ok(Box::new(BleConnection { device }))
    }
}
