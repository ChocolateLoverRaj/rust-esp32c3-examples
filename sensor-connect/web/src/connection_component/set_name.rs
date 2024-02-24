use common::validate_short_name::SHORT_NAME_MAX_LENGTH;
use leptos::{
    component, create_effect, create_node_ref, ev,
    html::{button, input, td, Input},
    Action, Fragment, IntoView, NodeRef, SignalGet,
};

#[component]
pub fn SetName<Close: Fn() + Copy + 'static>(
    initial_name: String,
    set_name: Action<String, ()>,
    close: Close,
) -> impl IntoView {
    let input_element: NodeRef<Input> = create_node_ref();
    let is_changing_name = set_name.pending();
    let version = set_name.version();
    create_effect(move |previous_version| {
        let version = version.get();
        if let Some(previous_version) = previous_version {
            if version == previous_version + 1 {
                close();
            }
        }
        version
    });
    let disabled = move || is_changing_name.get();

    Fragment::new(vec![
        td().child(
            input()
                .attr("placeholder", "New name")
                .attr("maxlength", SHORT_NAME_MAX_LENGTH)
                .attr("value", initial_name)
                .attr("disabled", disabled)
                .node_ref(input_element),
        )
        .into_view(),
        td().child(Fragment::new(vec![
            button()
                .child("Set")
                .on(ev::click, move |_e| {
                    set_name.dispatch(input_element.get().unwrap().value())
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
