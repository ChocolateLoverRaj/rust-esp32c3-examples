use std::{fmt::Debug, future::Future};

use wasm_bindgen::JsValue;

pub trait Connection: Debug {
    fn get_connection_type(&self) -> String;
    fn get_name<'a>(&'a self) -> Box<dyn Future<Output = String> + Unpin + 'a>;
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
