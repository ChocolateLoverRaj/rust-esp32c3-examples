use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{BluetoothRemoteGattCharacteristic, BluetoothRemoteGattService};

pub async fn get_characteristic(
    service: &BluetoothRemoteGattService,
    uuid: &str,
) -> BluetoothRemoteGattCharacteristic {
    JsFuture::from(service.get_characteristic_with_str(uuid))
        .await
        .unwrap()
        .dyn_into()
        .unwrap()
}
