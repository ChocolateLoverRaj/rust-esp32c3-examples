use std::fmt::Debug;

use wasm_bindgen::JsValue;

pub trait Connection: Sized + Debug {
    fn is_available() -> bool;
    async fn connect() -> Result<Self, JsValue>;
}
