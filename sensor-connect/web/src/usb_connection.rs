use common::Command;
use futures::{AsyncBufReadExt, StreamExt, TryStreamExt};
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::{js_sys::Uint8Array, JsFuture};
use web_sys::{
    window, ReadableStreamDefaultReader, SerialOptions, SerialPort, WritableStreamDefaultWriter,
};

use crate::{
    connection::{Connection, ConnectionBuilder},
    readable_stream::get_readable_stream,
};

#[derive(Debug)]
pub struct UsbConnection {
    read_stream: ReadableStreamDefaultReader,
    write_stream: WritableStreamDefaultWriter,
}
impl Connection for UsbConnection {
    fn get_connection_type(&self) -> String {
        "USB".into()
    }

    fn get_name<'a>(&'a self) -> Box<dyn std::future::Future<Output = String> + Unpin + 'a> {
        Box::new(Box::pin(async {
            let mut stream = Box::pin(get_readable_stream(&self.read_stream))
                .map(|v| Ok::<_, std::io::Error>(v))
                .into_async_read()
                .lines();
            JsFuture::from(
                self.write_stream.write_with_chunk(&Uint8Array::from(
                    format!(
                        "{}\n",
                        serde_json::to_string(&Command::ShortName(common::GetSet::Get)).unwrap()
                    )
                    .as_bytes(),
                )),
            )
            .await
            .unwrap();
            serde_json::from_str(&stream.next().await.unwrap().unwrap()).unwrap()
        }))
    }
}

#[derive(Debug)]
pub struct UsbConnectionBuilder {}

impl ConnectionBuilder for UsbConnectionBuilder {
    fn is_available() -> bool {
        !window().unwrap().navigator().serial().is_undefined()
    }

    async fn connect() -> Result<Box<dyn Connection>, JsValue> {
        let port: SerialPort =
            JsFuture::from(window().unwrap().navigator().serial().request_port())
                .await?
                .dyn_into()?;
        JsFuture::from(port.open(&SerialOptions::new(460800))).await?;
        let read_stream: ReadableStreamDefaultReader =
            port.readable().get_reader().dyn_into().unwrap();
        let write_stream: WritableStreamDefaultWriter = port.writable().get_writer()?;
        Ok(Box::new(UsbConnection {
            read_stream,
            write_stream,
        }))
    }
}
