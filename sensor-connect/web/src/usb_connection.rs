use std::fmt::Debug;

use ansitok::{ElementKind, parse_ansi};
use futures::{AsyncBufReadExt, StreamExt, TryStreamExt};
use futures_core::FusedStream;
use stream_broadcast::StreamBroadcastExt;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::{
    ReadableStreamDefaultReader, SerialOptions, SerialPort, window, WritableStreamDefaultWriter,
};

use common::MessageFromEsp;

use crate::{
    connection::{Connection, ConnectionBuilder},
    readable_stream::get_readable_stream,
};
use crate::connection::Characteristic;
use crate::usb_connection::message_writer::MessageWriter;
use crate::usb_connection::name_messenger::NameMessenger;
use crate::usb_connection::usb_characteristic::UsbCharacteristic;

mod usb_characteristic;
mod usb_characteristic_messenger;
mod name_messenger;
mod message_writer;

pub struct UsbConnection<T: FusedStream<Item=MessageFromEsp> + Sized + Unpin + 'static> {
    name_characteristic: UsbCharacteristic<String, NameMessenger, T>,
}

impl<T: FusedStream<Item=MessageFromEsp> + StreamBroadcastExt + Sized + Unpin + 'static>
Connection for UsbConnection<T>
{
    fn get_connection_type(&self) -> String {
        "USB".into()
    }

    fn name(&self) -> Box<dyn Characteristic<String>> {
        Box::new(self.name_characteristic.clone())
    }
}

impl<T: FusedStream<Item=MessageFromEsp> + StreamBroadcastExt + Sized + Unpin + 'static> Debug
for UsbConnection<T>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "USB Connection")
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

        let message_stream = Box::pin(
            get_readable_stream(read_stream.clone())
                .map(|v| Ok::<_, std::io::Error>(v))
                .into_async_read()
                .lines()
                .map(|line| line.unwrap())
                .filter(|line| {
                    let is_empty = line.is_empty();
                    async move { !is_empty }
                })
                // Sometimes there is an info line without a \n that becomes part of the first message
                .map(|line| match line.find("{") {
                    Some(pos) => line.split_at(pos).1.to_owned(),
                    None => line,
                })
                .filter(|line| {
                    let result = parse_ansi(line)
                        .find(|part| match part.kind() {
                            ElementKind::Text => false,
                            _ => true,
                        })
                        .is_none();
                    async move { result }
                })
                .map(|line| {
                    let message_from_esp: MessageFromEsp = serde_json::from_str(&line).unwrap();
                    message_from_esp
                }),
        )
            .fuse()
            .broadcast_unlimited();

        Ok(Box::new(UsbConnection {
            name_characteristic: UsbCharacteristic::new(message_stream.clone(), MessageWriter::new(write_stream.clone())),
        }))
    }
}
