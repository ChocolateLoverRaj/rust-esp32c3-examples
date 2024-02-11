use wasm_react::{h, hooks::use_state, props::Style, Callback, Component, VNode};

pub struct Counter {
    pub initial_counter: i32,
}

impl Component for Counter {
    fn render(&self) -> VNode {
        let counter = use_state(|| self.initial_counter);

        let vnode = h!(div).build((
            h!(p)
                .style(&Style::new().color("green"))
                .build(("Counter: ", *counter.value())),
            h!(button)
                .on_click(&Callback::new({
                    let mut counter = counter.clone();
                    move |_| counter.set(|c| c + 1)
                }))
                .build("+1"),
            h!(button)
                .on_click(&Callback::new({
                    let mut counter = counter.clone();
                    move |_| counter.set(|c| c - 1)
                }))
                .build("-1"),
        ));
        vnode
    }
}
