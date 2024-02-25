use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{BluetoothRemoteGattCharacteristic, BluetoothRemoteGattService};

use common::SHORT_NAME_UUID;

pub async fn get_short_name_characteristic(
    service: &BluetoothRemoteGattService,
) -> BluetoothRemoteGattCharacteristic {
    JsFuture::from(service.get_characteristic_with_str(SHORT_NAME_UUID))
        .await
        .unwrap()
        .dyn_into()
        .unwrap()
}
