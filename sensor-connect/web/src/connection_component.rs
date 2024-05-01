use std::rc::Rc;

use leptos::html::{table, tbody, td, th, tr};
use leptos::{component, IntoView, MaybeSignal, SignalGet};

use name_component::{NameComponent, NameComponentProps};
use set_name::{SetName, SetNameProps};

use crate::connection::Connection;
use crate::connection_component::ble_on_component::{BleOnComponent, BleOnComponentProps};
use crate::connection_component::distance_component::{DistanceComponent, DistanceComponentProps};
use crate::connection_component::ir_component::{IrComponent, IrComponentProps};
use crate::connection_component::passkey_component::{PasskeyComponent, PasskeyComponentProps};

mod name_component;
mod passkey_component;
mod set_name;
mod set_passkey;
mod ble_on_component;
mod ir_component;
mod distance_component;

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
            }))
            .child(BleOnComponent(BleOnComponentProps {
                characteristic: Rc::new(connection.get().ble_on())
            }))
            .child(IrComponent(IrComponentProps {
                characteristic: connection.get().get_ir_led_characteristic().map(|characteristic| Rc::new(characteristic))
            }))
            .child(DistanceComponent(DistanceComponentProps{
                characteristic: connection.get().get_distance_characteristic().map(|characteristic| Rc::new(characteristic))
            })),
    )
}
