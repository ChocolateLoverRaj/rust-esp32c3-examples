use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{BluetoothDevice, BluetoothRemoteGattService};

use common::SERVICE_UUID;

/// Gets the Sensor Connect BLE service
pub async fn get_service(device: &BluetoothDevice) -> BluetoothRemoteGattService {
    JsFuture::from(
        device
            .gatt()
            .unwrap()
            .get_primary_service_with_str(SERVICE_UUID),
    )
    .await
    .unwrap()
    .dyn_into()
    .unwrap()
}
