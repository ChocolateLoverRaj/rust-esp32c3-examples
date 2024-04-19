use std::rc::Rc;

use futures::lock::Mutex;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::js_sys::Uint8Array;
use wasm_bindgen_futures::JsFuture;
use web_sys::WritableStreamDefaultWriter;

use common::MessageToEsp;

#[derive(Clone, Debug)]
pub struct MessageWriter {
    writer: Rc<Mutex<WritableStreamDefaultWriter>>,
}

impl MessageWriter {
    pub fn new(writer: WritableStreamDefaultWriter) -> Self {
        Self {
            writer: Rc::new(Mutex::new(writer)),
        }
    }
    pub async fn write(&self, message: &MessageToEsp) -> Result<(), JsValue> {
        let write_stream = self.writer.lock().await;
        JsFuture::from(write_stream.write_with_chunk(&Uint8Array::from(
            format!("{}\n", serde_json::to_string(message).unwrap()).as_bytes(),
        )))
        .await?;
        Ok(())
    }
}
