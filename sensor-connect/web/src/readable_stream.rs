use futures::stream::unfold;
use futures_core::Stream;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::{
    js_sys::{IteratorNext, Uint8Array},
    JsFuture,
};
use web_sys::{console, ReadableStreamDefaultReader};

pub fn get_readable_stream<'a>(
    read_stream: &'a ReadableStreamDefaultReader,
) -> impl Stream<Item = Vec<u8>> + 'a {
    unfold((), |_| async {
        let iterator_next: IteratorNext = JsFuture::from(read_stream.read())
            .await
            .unwrap()
            .unchecked_into();
        let s: Uint8Array = iterator_next.value().dyn_into().unwrap();

        console::log_1(&s);
        Some((s.to_vec(), ()))
    })
}
