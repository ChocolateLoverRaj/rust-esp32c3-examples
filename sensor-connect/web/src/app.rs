use std::sync::{Arc, Mutex};

use wasm_bindgen::JsValue;
use wasm_react::{callback, hooks::use_state, Component, VNode};

use crate::{connection_enum::ConnectionEnum, connection_options::ConnectionOptions};

pub struct App;

impl Component for App {
    fn render(&self) -> VNode {
        let connection = use_state(|| None::<ConnectionEnum>);

        ConnectionOptions {
            on_connect: {
                let mut connection = connection.clone();
                move |new_connection| {
                    // connection.set(|_| Some(new_connection));
                }
            },
        }
        .build()
    }
}

impl TryFrom<JsValue> for App {
    type Error = JsValue;

    fn try_from(_: JsValue) -> Result<Self, Self::Error> {
        console_error_panic_hook::set_once();
        Ok(App)
    }
}
