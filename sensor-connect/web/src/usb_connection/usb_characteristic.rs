use std::fmt::{Debug, Formatter};
use std::future::Future;
use std::marker::PhantomData;

use futures::stream::unfold;
use futures::StreamExt;
use futures_core::FusedStream;
use leptos::{create_signal_from_stream, ReadSignal};
use stream_broadcast::StreamBroadcastUnlimited;

use common::{Message, MessageFromEsp, MessageToEsp, ResponseData};

use crate::connection::Characteristic;
use crate::usb_connection::message_writer::MessageWriter;
use crate::usb_connection::usb_characteristic_messenger::UsbCharacteristicMessenger;

pub struct UsbCharacteristic<T, M: UsbCharacteristicMessenger<T>, S: FusedStream<Item=MessageFromEsp> + Sized + Unpin + 'static> {
    _phantom_data_t: PhantomData<T>,
    _phantom_data_m: PhantomData<M>,
    message_writer: MessageWriter,
    message_stream: StreamBroadcastUnlimited<S>,
}

impl<T, M: UsbCharacteristicMessenger<T>, S: FusedStream<Item=MessageFromEsp> + Sized + Unpin + 'static> Clone for UsbCharacteristic<T, M, S> {
    fn clone(&self) -> Self {
        Self {
            _phantom_data_m: self._phantom_data_m.clone(),
            _phantom_data_t: self._phantom_data_t.clone(),
            message_writer: self.message_writer.clone(),
            message_stream: self.message_stream.clone(),
        }
    }
}

impl<T, M: UsbCharacteristicMessenger<T>, S: FusedStream<Item=MessageFromEsp> + Sized + Unpin + 'static> Debug for UsbCharacteristic<T, M, S> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "USB Characteristic")
    }
}

impl<T, M: UsbCharacteristicMessenger<T>, S: FusedStream<Item=MessageFromEsp> + Sized + Unpin + 'static> Characteristic<T> for UsbCharacteristic<T, M, S> {
    fn watch(&self) -> ReadSignal<Option<T>> {
        let message_writer = self.message_writer.clone();
        let message_stream = self.message_stream.clone();

        let stream_broadcast_unlimited = self.message_stream.clone();

        create_signal_from_stream(Box::pin(
            unfold(Some((message_writer, message_stream)), |first| async move {
                match first {
                    Some((write_stream, message_stream)) => {
                        Some((Self::get_name(write_stream, message_stream).await, None))
                    }
                    None => None,
                }
            })
                .chain({
                    let message_writer = self.message_writer.clone();
                    let message_stream = self.message_stream.clone();

                    stream_broadcast_unlimited.filter_map(move |(_id, message)| {
                        let write_stream = message_writer.clone();
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
                })))
    }

    fn set(&self, new_value: T) -> Box<dyn Future<Output=()> + Unpin> {
        let message_to_esp = MessageToEsp::new(M::create_set_request(new_value));
        let mut characteristic = self.clone();
        Box::new(Box::pin(async move {
            characteristic.message_writer.write(&message_to_esp).await.unwrap();
            loop {
                let (_id, message) = characteristic.message_stream.next().await.unwrap();
                match message {
                    MessageFromEsp::Response(response) => {
                        if response.id == message_to_esp.id {
                            match response.data {
                                ResponseData::Complete => return,
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

impl<T, M: UsbCharacteristicMessenger<T>, S: FusedStream<Item=MessageFromEsp> + Sized + Unpin + 'static> UsbCharacteristic<T, M, S> {
    fn get_name(
        message_writer: MessageWriter,
        mut message_stream: StreamBroadcastUnlimited<S>,
    ) -> Box<dyn std::future::Future<Output=T> + Unpin> {
        Box::new(Box::pin(async move {
            let message_to_esp = MessageToEsp::new(M::create_get_request());
            message_writer.write(&message_to_esp).await.unwrap();
            loop {
                let (_id, message) = message_stream.next().await.unwrap();
                match message {
                    MessageFromEsp::Response(response) => {
                        if response.id == message_to_esp.id {
                            match M::find_get_response(response.data) {
                                Some(value) => return value,
                                _ => {}
                            }
                        }
                    }
                    _ => {}
                }
            }
        }))
    }

    pub fn new(message_stream: StreamBroadcastUnlimited<S>, message_writer: MessageWriter) -> Self {
        Self {
            _phantom_data_t: Default::default(),
            _phantom_data_m: Default::default(),
            message_stream,
            message_writer,
        }
    }
}