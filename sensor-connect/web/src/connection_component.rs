use std::rc::Rc;

use leptos::{
    component, create_action, create_memo, create_rw_signal, IntoView, MaybeSignal, SignalGet,
    SignalSet, SignalWith, view,
};

use name_component::{NameComponent, NameComponentProps};
use set_name::{SetName, SetNameProps};

use crate::connection::Connection;

mod set_name;
mod name_component;

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

                    <td>{{
            let connection = connection.clone(); move
                     || connection().get_connection_type()
        }}</td>
                </tr>
            {NameComponent(NameComponentProps { name_characteristic: Rc::new(connection.get().name())})}
            </tbody>
        </table>
    }
}
