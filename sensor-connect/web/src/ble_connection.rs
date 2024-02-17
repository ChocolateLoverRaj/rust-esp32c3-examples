use common::SERVICE_UUID;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::{
    js_sys::{Array, JsString, Object},
    window, BluetoothDevice, RequestDeviceOptions,
};

use crate::connection::Connection;

#[derive(Debug)]
pub struct BleConnection {
    device: BluetoothDevice,
}

impl Connection for BleConnection {
    fn is_available() -> bool {
        window().unwrap().navigator().bluetooth().is_some()
    }

    async fn connect() -> Result<Self, JsValue> {
        // FIXME: Error handling
        let device = JsFuture::from(
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
        Ok(Self { device })
    }
}
