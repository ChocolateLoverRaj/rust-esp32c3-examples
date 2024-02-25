use std::rc::Rc;

use leptos::{
    component, create_action, create_memo, create_rw_signal, IntoView, MaybeSignal, SignalGet,
    SignalSet, SignalWith, view,
};

use set_name::{SetName, SetNameProps};

use crate::connection::Connection;

mod set_name;

#[component]
pub fn ConnectionComponent(connection: MaybeSignal<Rc<Box<dyn Connection>>>) -> impl IntoView {
    let name = connection.get().name().watch();
    let set_name = create_action({
        let connection = connection.clone();
        move |new_name: &String| connection.get().name().set(new_name.to_owned())
    });
    let changing_name = create_rw_signal(false);
    let is_changing_name = create_memo(move |_| changing_name.get());

    view! {
        <table>
            <tbody>
                <tr>
                    <th>Connection Type</th>

                    <td>{move || connection().get_connection_type()}</td>
                </tr>
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
            </tbody>
        </table>
    }
}
