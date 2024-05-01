use wasm_bindgen_futures::js_sys::{DataView, Uint8Array};

use common::ir_data::IrData;

use crate::ble_connection::ble_serializer::BleSerializer;

#[derive(Clone, Debug)]
pub struct IrDataSerializer;

impl BleSerializer<IrData> for IrDataSerializer {
    fn serialize(data: IrData) -> Vec<u8> {
        data.to_bytes().into()
    }

    fn deserialize(data: DataView) -> IrData {
        let vec = Uint8Array::new(&data.buffer()).to_vec();
        IrData::from_bytes(vec.try_into().unwrap())
    }
}
