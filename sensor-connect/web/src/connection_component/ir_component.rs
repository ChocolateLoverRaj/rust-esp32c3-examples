use std::rc::Rc;
use leptos::{component, IntoView, SignalGet};
use leptos::html::{i, td, th, tr};
use common::ir_data::IrData;
use crate::connection::Characteristic;

#[component]
pub fn ir_component(characteristic: Option<Rc<Box<dyn Characteristic<IrData>>>>) -> impl IntoView {
    tr()
        .child(th().child("IR State"))
        .child(td().child(match characteristic {
            Some(characteristic) => {
                let signal = characteristic.watch();
                (move || match signal.get() {
                    Some(ir_data) => {
                        match ir_data.is_receiving_light {
                            true => "Unblocked",
                            false => "Blocked"
                        }.into_view()
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