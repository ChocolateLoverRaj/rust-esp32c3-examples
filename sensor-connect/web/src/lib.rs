use counter::Counter;
use wasm_bindgen::JsValue;
use wasm_react::{export_components, Component, VNode};
mod counter;

pub struct App;

impl Component for App {
    fn render(&self) -> VNode {
        Counter { initial_counter: 0 }.build()
    }
}

impl TryFrom<JsValue> for App {
    type Error = JsValue;

    fn try_from(_: JsValue) -> Result<Self, Self::Error> {
        Ok(App)
    }
}

export_components! { App }
