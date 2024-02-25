use wasm_bindgen_futures::js_sys::{DataView, Uint8Array};

use crate::ble_connection::ble_serializer::BleSerializer;

#[derive(Clone, Debug)]
pub struct BleStringSerializer;

impl BleSerializer<String> for BleStringSerializer {
    fn serialize(data: String) -> Vec<u8> {
        data.as_bytes().to_owned()
    }

    fn deserialize(data: DataView) -> String {
        String::from_utf8(
            Uint8Array::new(&data.buffer()).to_vec(),
        ).unwrap()
    }
}

impl Default for BleStringSerializer {
    fn default() -> Self {
        Self {}
    }
}