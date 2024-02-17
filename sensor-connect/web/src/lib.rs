use app::App;
use leptos::*;
use wasm_bindgen::prelude::*;

mod app;
mod connection_options;

#[wasm_bindgen(start)]
pub fn main() -> Result<(), JsValue> {
    console_error_panic_hook::set_once();
    mount_to_body(|| view! { <App/> });

    Ok(())
}
