use wasm_bindgen_futures::js_sys::{DataView, Uint8Array};

use crate::ble_connection::ble_serializer::BleSerializer;

#[derive(Clone, Debug)]
pub struct BleU32Serializer;

impl BleSerializer<u32> for BleU32Serializer {
    fn serialize(data: u32) -> Vec<u8> {
        data.to_be_bytes().into()
    }

    fn deserialize(data: DataView) -> u32 {
        u32::from_be_bytes(Uint8Array::new(&data.buffer()).to_vec().try_into().unwrap())
    }
}

impl Default for BleU32Serializer {
    fn default() -> Self {
        Self {}
    }
}