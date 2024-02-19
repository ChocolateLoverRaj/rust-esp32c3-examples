use futures::stream::unfold;
use futures_core::{FusedStream, Stream};
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::{
    js_sys::{IteratorNext, Uint8Array},
    JsFuture,
};
use web_sys::{console, ReadableStreamDefaultReader};

pub fn get_readable_stream(
    read_stream: ReadableStreamDefaultReader,
) -> impl FusedStream<Item = Vec<u8>> + Sized {
    unfold(read_stream, |read_stream| async {
        let iterator_next: IteratorNext = JsFuture::from(read_stream.read())
            .await
            .unwrap()
            .unchecked_into();
        let s: Uint8Array = iterator_next.value().dyn_into().unwrap();

        console::log_1(&s);
        Some((s.to_vec(), read_stream))
    })
}
