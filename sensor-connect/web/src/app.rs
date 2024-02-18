use std::rc::Rc;

use leptos::{component, create_action, view, Callback, IntoView, SignalWith};
use wasm_bindgen_test::console_log;

use crate::{
    ble_connection::BleConnectionBuilder, connection::ConnectionBuilder,
    connection_component::ConnectionComponent, connection_options::ConnectionOptions,
    connection_type::ConnectionType, usb_connection::UsbConnectionBuilder,
};

#[component]
pub fn App() -> impl IntoView {
    let connect = create_action(|connection_type: &ConnectionType| {
        let connection_type = connection_type.to_owned();
        async move {
            let result = match connection_type {
                ConnectionType::Usb => UsbConnectionBuilder::connect().await,
                ConnectionType::Ble => BleConnectionBuilder::connect().await,
            }
            .map(|b| Rc::new(b));
            if let Err(e) = result.as_ref() {
                console_log!("Error connecting with {:#?}: {:#?}", connection_type, e);
            }
            result
        }
    });

    let connection = connect.value();

    view! {
        {move || {
            connection
                .with(|connection| {
                    match connection.as_ref().map(|result| result.as_ref().ok()).flatten() {
                        Some(connection) => {
                            view! { <ConnectionComponent connection=connection.clone().into()/> }
                        }
                        None => {
                            view! {
                                <ConnectionOptions on_click_connect=Callback::new(move |
                                    connection_type|
                                { connect.dispatch(connection_type) })/>
                            }
                        }
                    }
                })
        }}
    }
}
