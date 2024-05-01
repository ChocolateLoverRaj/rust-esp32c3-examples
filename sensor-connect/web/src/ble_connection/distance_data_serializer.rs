use wasm_bindgen_futures::js_sys::{DataView, Uint8Array};
use common::distance_data::DistanceData;

use crate::ble_connection::ble_serializer::BleSerializer;

#[derive(Clone, Debug)]
pub struct DistanceDataSerializer;

impl BleSerializer<DistanceData> for DistanceDataSerializer {
    fn serialize(data: DistanceData) -> Vec<u8> {
        data.to_bytes().into()
    }

    fn deserialize(data: DataView) -> DistanceData {
        let vec = Uint8Array::new(&data.buffer()).to_vec();
        DistanceData::from_bytes(vec.try_into().unwrap())
    }
}
