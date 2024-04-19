use std::rc::Rc;

use leptos::{
    component, create_action, create_memo, create_rw_signal, view, IntoView, SignalGet, SignalSet,
    SignalWith,
};

use crate::{
    connection::Characteristic,
    connection_component::{SetName, SetNameProps},
};

#[component]
pub fn NameComponent(name_characteristic: Rc<Box<dyn Characteristic<String>>>) -> impl IntoView {
    let name = name_characteristic.watch();
    let set_name = create_action({
        let name_characteristic = name_characteristic.clone();
        move |new_name: &String| name_characteristic.set(new_name.to_owned())
    });
    let changing_name = create_rw_signal(false);
    let is_changing_name = create_memo(move |_| changing_name.get());

    view! {
                <tr>
                    <th>Device Name</th>
                    {move || match is_changing_name() {
                        true => {
                            SetName(SetNameProps {
                                    initial_name: name.get().unwrap_or_default(),
                                    set_name,
                                    close: move || changing_name.set(false),
                                })
                                .into_view()
                        }
                        false => {
                            view! {
                                <td>
                                    {move || {
                                        name.with(|name| match name {
                                                None => "Loading".into(),
                                                Some(name) => name.to_owned(),
                                            })
                                            .into_view()
                                    }}

                                </td>
                                <td>
                                    <button on:click=move |_ev| {
                                        changing_name.set(true)
                                    }>Edit</button>

                                </td>
                            }
                                .into_view()
                        }
                    }}

                </tr>
    }
}
