use std::fmt::Debug;

use ansitok::{parse_ansi, ElementKind};
use common::{CommandData, Message, MessageFromEsp, MessageToEsp};
use futures::{AsyncBufReadExt, StreamExt, TryStreamExt};
use futures_core::FusedStream;
use leptos::{create_signal_from_stream, ReadSignal};
use stream_broadcast::{StreamBroadcastExt, StreamBroadcastUnlimited};
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::{js_sys::Uint8Array, JsFuture};
use web_sys::{
    console, window, ReadableStreamDefaultReader, SerialOptions, SerialPort,
    WritableStreamDefaultWriter,
};

use crate::{
    connection::{Connection, ConnectionBuilder},
    readable_stream::get_readable_stream,
};

pub struct UsbConnection<T: FusedStream<Item = MessageFromEsp> + Sized + Unpin + 'static> {
    message_stream: StreamBroadcastUnlimited<T>,
    write_stream: WritableStreamDefaultWriter,
}
impl<T: FusedStream<Item = MessageFromEsp> + StreamBroadcastExt + Sized + Unpin + 'static>
    Connection for UsbConnection<T>
{
    fn get_connection_type(&self) -> String {
        "USB".into()
    }

    fn get_name<'a>(&'a self) -> Box<dyn std::future::Future<Output = String> + Unpin + 'a> {
        Box::new(Box::pin(self.get_name()))
    }
}
impl<T: FusedStream<Item = MessageFromEsp> + StreamBroadcastExt + Sized + Unpin + 'static> Debug
    for UsbConnection<T>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "USB Connection")
    }
}

impl<T: FusedStream<Item = MessageFromEsp> + StreamBroadcastExt + Sized + Unpin + 'static>
    UsbConnection<T>
{
    async fn get_name(&self) -> String {
        let message_to_esp = MessageToEsp::new(CommandData::ShortName(common::GetSet::Get));
        JsFuture::from(self.write_stream.write_with_chunk(&Uint8Array::from(
            format!("{}\n", serde_json::to_string(&message_to_esp).unwrap()).as_bytes(),
        )))
        .await
        .unwrap();
        let mut stream = self.message_stream.clone();
        loop {
            let (_id, message) = stream.next().await.unwrap();
            match message {
                MessageFromEsp::Response(response) => {
                    if response.id == message_to_esp.id {
                        match response.data {
                            common::ResponseData::GetShortName(name) => return name,
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        }
    }

    pub fn watch_name(&'static self) -> ReadSignal<Option<String>> {
        let a: ReadSignal<Option<String>> =
            create_signal_from_stream(Box::pin(self.message_stream.clone().filter_map(
                move |(_id, message)| async move {
                    match message {
                        MessageFromEsp::Event(event) => match event {
                            Message::ShortNameChange => Some(self.get_name().await),
                            _ => None,
                        },
                        MessageFromEsp::Response(_) => None,
                    }
                },
            )));
        a
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
            message_stream,
            write_stream,
        }))
    }
}
