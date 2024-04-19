use leptos::leptos_dom::logging::console_log;
use wasm_bindgen_futures::js_sys::{DataView, Uint8Array};
use wasm_bindgen_test::console_log;

use crate::ble_connection::ble_serializer::BleSerializer;

#[derive(Clone, Debug)]
pub struct BleBoolSerializer;

impl BleSerializer<bool> for BleBoolSerializer {
    fn serialize(data: bool) -> Vec<u8> {
        u8::from(data).to_be_bytes().into()
    }

    fn deserialize(data: DataView) -> bool {
        match u8::from_be_bytes(Uint8Array::new(&data.buffer()).to_vec().try_into().unwrap()) {
            1 => true,
            0 => false,
            _ => panic!("Invalid byte")
        }
    }
}
