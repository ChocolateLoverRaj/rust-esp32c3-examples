use common::validate_short_name::SHORT_NAME_MAX_LENGTH;
use leptos::{
    component, create_effect, create_node_ref, ev,
    html::{button, input, td, Input},
    Action, Fragment, IntoView, NodeRef, SignalGet,
};

#[component]
pub fn SetPasskey<Close: Fn() + Copy + 'static>(
    initial_passkey: u32,
    set_passkey: Action<u32, ()>,
    close: Close,
) -> impl IntoView {
    let input_element: NodeRef<Input> = create_node_ref();
    let is_changing_passkey = set_passkey.pending();
    let version = set_passkey.version();
    create_effect(move |previous_version| {
        let version = version.get();
        if let Some(previous_version) = previous_version {
            if version == previous_version + 1 {
                close();
            }
        }
        version
    });
    let disabled = move || is_changing_passkey.get();

    Fragment::new(vec![
        td().child(
            input()
                .attr("type", "number")
                .attr("min", 0)
                .attr("max", 999999)
                .attr("value", initial_passkey)
                .attr("disabled", disabled)
                .node_ref(input_element),
        )
        .into_view(),
        td().child(Fragment::new(vec![
            button()
                .child("Set")
                .on(ev::click, move |_e| {
                    set_passkey.dispatch(input_element.get().unwrap().value_as_number() as u32)
                })
                .attr("disabled", disabled)
                .into_view(),
            {
                button()
                    .child("Cancel")
                    .attr("disabled", disabled)
                    .on(ev::click, move |_e| close())
            }
            .into_view(),
        ]))
        .into_view(),
    ])
}
