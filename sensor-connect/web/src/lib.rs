use app::App;
use leptos::*;
use wasm_bindgen::prelude::*;

mod app;
mod ble_connection;
mod connection;
mod connection_component;
mod connection_options;
mod connection_type;
mod readable_stream;
mod usb_connection;

#[wasm_bindgen(start)]
pub fn main() -> Result<(), JsValue> {
    console_error_panic_hook::set_once();
    mount_to_body(|| view! { <App/> });

    Ok(())
}
