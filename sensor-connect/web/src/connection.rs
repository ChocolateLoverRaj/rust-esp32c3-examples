use std::{fmt::Debug, future::Future};

use leptos::ReadSignal;
use wasm_bindgen::JsValue;
use common::distance_data::DistanceData;
use common::ir_data::IrData;

pub trait Characteristic<T> {
    fn watch(&self) -> ReadSignal<Option<T>>;
    fn set(&self, new_value: T) -> Box<dyn Future<Output = ()> + Unpin>;
}

pub trait Connection: Debug {
    fn get_connection_type(&self) -> String;
    fn name(&self) -> Box<dyn Characteristic<String>>;
    fn passkey(&self) -> Box<dyn Characteristic<u32>>;
    fn ble_on(&self) -> Box<dyn Characteristic<bool>>;
    fn get_ir_led_characteristic(&self) -> Option<Box<dyn Characteristic<IrData>>>;
    fn get_distance_characteristic(&self) -> Option<Box<dyn Characteristic<DistanceData>>>;
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