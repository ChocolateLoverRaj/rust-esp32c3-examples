use std::rc::Rc;

use leptos::html::{table, tbody, td, th, tr};
use leptos::{component, IntoView, MaybeSignal, SignalGet, SignalSet, SignalWith};

use name_component::{NameComponent, NameComponentProps};
use set_name::{SetName, SetNameProps};

use crate::connection::Connection;
use crate::connection_component::passkey_component::{PasskeyComponent, PasskeyComponentProps};

mod name_component;
mod passkey_component;
mod set_name;
mod set_passkey;

#[component]
pub fn ConnectionComponent(connection: MaybeSignal<Rc<Box<dyn Connection>>>) -> impl IntoView {
    table().child(
        tbody()
            .child(tr().child(th().child("Connection Type")).child(td().child({
                let connection = connection.clone();
                move || connection().get_connection_type()
            })))
            .child(NameComponent(NameComponentProps {
                name_characteristic: Rc::new(connection.get().name()),
            }))
            .child(PasskeyComponent(PasskeyComponentProps {
                passkey_characteristic: Rc::new(connection.get().passkey()),
            })),
    )
}
