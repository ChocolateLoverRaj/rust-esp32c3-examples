use std::rc::Rc;
use leptos::{component, IntoView, SignalGet};
use leptos::html::{i, td, th, tr};
use common::distance_data::DistanceData;
use common::ir_data::IrData;
use crate::connection::Characteristic;

#[component]
pub fn distance_component(characteristic: Option<Rc<Box<dyn Characteristic<DistanceData>>>>) -> impl IntoView {
    tr()
        .child(th().child("Distance"))
        .child(td().child(match characteristic {
            Some(characteristic) => {
                let signal = characteristic.watch();
                (move || match signal.get() {
                    Some(ir_data) => {
                        format!("{}mm", ir_data.distance).into_view()
                    },
                    None => {
                        "Loading".into_view()
                    }
                }).into_view()
            },
            None => {
                i().child("Not connected").into_view()
            }
        }))
}