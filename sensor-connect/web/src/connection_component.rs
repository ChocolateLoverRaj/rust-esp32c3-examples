use crate::connection::Connection;
use leptos::{component, create_resource, view, IntoView, MaybeSignal, SignalWith};
use std::rc::Rc;

#[component]
pub fn ConnectionComponent(connection: MaybeSignal<Rc<Box<dyn Connection>>>) -> impl IntoView {
    let name = create_resource(connection.clone(), |connection| async move {
        connection.get_name().await
    });
    view! {
        <table>
            <tbody>
                <tr>
                    <th>Connection Type</th>

                    <td>{move || connection().get_connection_type()}</td>
                </tr>
                <tr>
                    <th>Device Name</th>

                    <td>
                        {move || {
                            name.with(|name| match name {
                                None => "Loading".into(),
                                Some(name) => name.to_owned(),
                            })
                        }}

                    </td>
                </tr>
            </tbody>
        </table>
    }
}
