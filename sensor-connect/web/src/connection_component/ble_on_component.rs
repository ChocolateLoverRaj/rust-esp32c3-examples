use std::rc::Rc;

use leptos::{component, ev, IntoView, SignalGet};
use leptos::html::{input, td, th, tr};
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use web_sys::HtmlInputElement;

use crate::connection::Characteristic;

#[component]
pub fn BleOnComponent(characteristic: Rc<Box<dyn Characteristic<bool>>>) -> impl IntoView {
    let ble_on = characteristic.watch();

    tr()
        .child(th().child("BLE On"))
        .child(td().child(move || {
            let characteristic = characteristic.clone();
            match ble_on.get() {
                Some(ble_on) => input()
                    .attr("type", "checkbox")
                    .attr("checked", ble_on)
                    .on(ev::input, move |e| {
                        let target: HtmlInputElement = e.target().unwrap().dyn_into().unwrap();
                        spawn_local(characteristic.set(target.checked()));
                    })
                    .into_view(),
                None => "Loading".into_view()
            }
        }))
        .child(td())
        .child("Turn BLE (Bluetooth Low Energy) on or off. If you accidentally turn it off while connected to BLE you will be disconnect and you'll have to connect with USB to turn it back on.")
}