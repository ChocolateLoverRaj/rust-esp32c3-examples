use std::fmt::Debug;

use ansitok::{parse_ansi, ElementKind};
use common::{CommandData, Message, MessageFromEsp, MessageToEsp};
use futures::{stream::unfold, AsyncBufReadExt, StreamExt, TryStreamExt};
use futures_core::FusedStream;
use leptos::{create_signal_from_stream, ReadSignal};
use stream_broadcast::{StreamBroadcastExt, StreamBroadcastUnlimited};
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::{js_sys::Uint8Array, JsFuture};
use web_sys::{
    window, ReadableStreamDefaultReader, SerialOptions, SerialPort, WritableStreamDefaultWriter,
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

    fn get_name(&self) -> Box<dyn std::future::Future<Output = String> + Unpin> {
        Self::get_name(self.write_stream.clone(), self.message_stream.clone())
    }

    fn watch_name(&self) -> ReadSignal<Option<String>> {
        let write_stream = self.write_stream.clone();
        let message_stream = self.message_stream.clone();

        let stream_broadcast_unlimited: StreamBroadcastUnlimited<T> = self.message_stream.clone();

        let a: ReadSignal<Option<String>> = create_signal_from_stream(Box::pin(
            unfold(Some((write_stream, message_stream)), |first| async move {
                match first {
                    Some((write_stream, message_stream)) => {
                        Some((Self::get_name(write_stream, message_stream).await, None))
                    }
                    None => None,
                }
            })
            .chain({
                let write_stream = self.write_stream.clone();
                let message_stream = self.message_stream.clone();

                stream_broadcast_unlimited.filter_map(move |(_id, message)| {
                    let write_stream = write_stream.clone();
                    let message_stream = message_stream.clone();

                    async move {
                        match message {
                            MessageFromEsp::Event(event) => match event {
                                Message::ShortNameChange => {
                                    Some(Self::get_name(write_stream, message_stream).await)
                                }
                                _ => None,
                            },
                            MessageFromEsp::Response(_) => None,
                        }
                    }
                })
            }),
        ));
        a
    }

    fn set_name(&self, new_name: &str) -> Box<dyn futures::prelude::Future<Output = ()> + Unpin> {
        let write_stream = self.write_stream.clone();
        let new_name = new_name.to_owned();
        let message_stream = self.message_stream.clone();
        let message_to_esp =
            MessageToEsp::new(CommandData::ShortName(common::GetSet::Set(new_name)));
        Box::new(Box::pin(async move {
            JsFuture::from(write_stream.write_with_chunk(&Uint8Array::from(
                format!("{}\n", serde_json::to_string(&message_to_esp).unwrap()).as_bytes(),
            )))
            .await
            .unwrap();
            Box::pin(message_stream.filter_map(|(_index, message)| async {
                match message {
                    MessageFromEsp::Response(response) => {
                        if response.id == message_to_esp.id {
                            match response.data {
                                common::ResponseData::Complete => Some(()),
                                _ => panic!(),
                            }
                        } else {
                            None
                        }
                    }
                    _ => None,
                }
            }))
            .next()
            .await
            .unwrap();
        }))
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
    fn get_name(
        write_stream: WritableStreamDefaultWriter,
        mut message_stream: StreamBroadcastUnlimited<T>,
    ) -> Box<dyn std::future::Future<Output = String> + Unpin> {
        Box::new(Box::pin(async move {
            let message_to_esp = MessageToEsp::new(CommandData::ShortName(common::GetSet::Get));
            JsFuture::from(write_stream.write_with_chunk(&Uint8Array::from(
                format!("{}\n", serde_json::to_string(&message_to_esp).unwrap()).as_bytes(),
            )))
            .await
            .unwrap();
            loop {
                let (_id, message) = message_stream.next().await.unwrap();
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
            message_stream,
            write_stream,
        }))
    }
}
