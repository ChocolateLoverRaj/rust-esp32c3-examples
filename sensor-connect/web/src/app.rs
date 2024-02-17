use leptos::{component, view, IntoView};

use crate::connection_options::ConnectionOptions;

#[component]
pub fn App() -> impl IntoView {
    view! { <ConnectionOptions/> }
}
