use std::{fmt::Debug, future::Future};

use leptos::ReadSignal;
use wasm_bindgen::JsValue;

pub trait Connection: Debug {
    fn get_connection_type(&self) -> String;
    fn get_name(&self) -> Box<dyn Future<Output = String> + Unpin>;
    fn watch_name(&self) -> ReadSignal<Option<String>>;
    // TODO: Return result and handle error
    fn set_name(&self, new_name: &str) -> Box<dyn Future<Output = ()> + Unpin>;
}

impl PartialEq<Box<dyn Connection>> for Box<dyn Connection> {
    fn eq(&self, other: &Box<dyn Connection>) -> bool {
        &self == &other
    }
}

pub trait ConnectionBuilder: Debug {
    fn is_available() -> bool;
    async fn connect() -> Result<Box<dyn Connection>, JsValue>;
}
