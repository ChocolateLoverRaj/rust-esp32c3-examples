use std::rc::Rc;

use common::INITIAL_PASSKEY;
use leptos::{component, create_action, create_node_ref, create_rw_signal, ev, IntoView, NodeRef, SignalGet, SignalSet};
use leptos::html::{button, code, input, Input, td, th, tr};
use wasm_bindgen_test::console_log;
use web_sys::window;
use zero_pad::zero_pad;

use crate::connection::Characteristic;

#[component]
pub fn PasskeyComponent(passkey_characteristic: Rc<Box<dyn Characteristic<u32>>>) -> impl IntoView {
    // let passkey = passkey_characteristic.watch();
    let set_passkey = create_action({
        let passkey_characteristic = passkey_characteristic.clone();
        move |new_passkey: &u32| passkey_characteristic.set(new_passkey.to_owned())
    });
    let changing_passkey = create_rw_signal(false);
    let is_viewing_passkey = create_rw_signal(false);
    // let is_changing_passkey = create_memo(move |_| changing_passkey.get());
    let input_element: NodeRef<Input> = create_node_ref();

    let disabled = move || set_passkey.pending();

    let get_passkey_text = move || {
        let signal = passkey_characteristic.watch();
        move || {
            signal.get().map(|passkey| zero_pad(passkey, 6))
        }
    };

    tr()
        .child(th().child("Passkey"))
        .child(td().child(move || match is_viewing_passkey.get() {
            true => match changing_passkey.get() {
                true => input()
                    .attr("placeholder", "Passkey")
                    .attr("minlength", 6)
                    .attr("maxlength", 6)
                    .attr("disabled", disabled())
                    .attr("value", get_passkey_text())
                    .node_ref(input_element)
                    .into_view(),
                false => code().child(get_passkey_text()).into_view()
            },
            false => "***".into_view()
        }))
        .child(td().child(move || match is_viewing_passkey.get() {
            true =>
                (
                    (button().child("Hide").on(ev::click, move |_e| {
                        is_viewing_passkey.set(false);
                    })),
                    (match changing_passkey.get() {
                        true => (
                            button().child("Cancel").on(ev::click, move |_e| {
                                changing_passkey.set(false);
                            }),
                            button().child("Save").on(ev::click, move |_e| {
                                match input_element.get().unwrap().value().parse::<u32>() {
                                    Ok(passkey) => {
                                        if passkey >= 0 && passkey <= 999999 {
                                            set_passkey.dispatch(passkey);
                                            // TODO: Error handling
                                            changing_passkey.set(false);
                                        } else {
                                            window().unwrap().alert_with_message("Invalid passkey number".into()).unwrap()
                                        }
                                    }
                                    Err(e) => {
                                        console_log!("Error parsing passkey: {:#?}", e);
                                        window().unwrap().alert_with_message("Invalid passkey".into()).unwrap()
                                    }
                                }
                            })
                        ).into_view(),
                        false => button().child("Edit").on(ev::click, move |_e| {
                            changing_passkey.set(true);
                        }).into_view()
                    })
                ).into_view(),
            false => button().child("Show").on(ev::click, move |_e| {
                is_viewing_passkey.set(true);
            }).into_view()
        }))
        .child(td()
            .child("A passkey is needed to change the name, passkey, or turn BLE on/off for the ESP32-C3. The initial passkey is ")
            .child(code().child(INITIAL_PASSKEY))
            .child(". If you don't know the passkey, you can change it without needing to know it by connecting with USB."))
}
