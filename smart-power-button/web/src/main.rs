use async_ui_web::{join, mount};
use async_ui_web::event_traits::EmitElementEvent;
use async_ui_web::html::{Br, Button};
use async_ui_web::shortcut_traits::{ShortcutRenderStr, UiFutureExt};
use dotenvy_macro::option_dotenv;
use futures::{SinkExt, StreamExt};
use gloo_console::{error, log};
use postcard::to_allocvec;
use stream_broadcast::StreamBroadcastExt;
use tokio::sync::Mutex;
use web_sys::window;
use ws_stream_wasm::{WsMessage, WsMeta};

use smart_power_button_common::{MessageToEsp, MessageToWeb};

use crate::stream_render_ext::StreamRenderExt;

mod web_socket_ext;
mod stream_render_ext;

fn main() {
    mount(app());
}

async fn app() {
    let ws_host = option_dotenv!("WS_HOST")
        .map_or(window().unwrap().location().host().unwrap(), |s| s.to_owned());
    let ws_url = format!("ws://{ws_host}");
    match WsMeta::connect(ws_url, None).meanwhile("Opening web socket".render()).await {
        Ok((_ws_meta, ws_stream)) => {
            let (w, r) = ws_stream.split();
            let w = Mutex::new(w);
            let message_to_web_stream = r
                .map(|message| match message {
                    WsMessage::Binary(data) => {
                        postcard::from_bytes::<MessageToWeb>(&data).unwrap()
                    }
                    WsMessage::Text(_) => unreachable!()
                })
                // This line is just to debug messages
                .inspect(|message| log!(format!("{message:?}")))
                .fuse()
                .broadcast(16);

            join((
                "Power LED Status: ".render(),
                message_to_web_stream
                    .clone()
                    .filter_map(|(_, message)| Box::pin(async move {
                        match message {
                            MessageToWeb::PowerLedStatus(is_on) => Some(is_on),
                            _ => None
                        }
                    }))
                    .map(|is_on| match is_on {
                        true => "On",
                        false => "Off"
                    }.render())
                    .render(),
                Br::new().render(),
                "HDD LED Status: ".render(),
                message_to_web_stream
                    .clone()
                    .filter_map(|(_, message)| Box::pin(async move {
                        match message {
                            MessageToWeb::HddLedStatus(is_on) => Some(is_on),
                            _ => None
                        }
                    }))
                    .map(|is_on| match is_on {
                        true => "On",
                        false => "Off"
                    }.render())
                    .render(),
                Br::new().render(),
                async {
                    let short_press_button = Button::new();
                    let long_press_button = Button::new();
                    join((
                        short_press_button.render("Press power button".render()),
                        Br::new().render(),
                        long_press_button.render("Press power button for a long time".render()),
                        async {
                            let mut stream = message_to_web_stream
                                .clone()
                                .filter_map(|(_, message)| Box::pin(async move {
                                    match message {
                                        MessageToWeb::PowerButtonStatus(is_on) => Some(is_on),
                                        _ => None
                                    }
                                }));
                            while let Some(is_on) = stream.next().await {
                                short_press_button.set_disabled(is_on);
                                long_press_button.set_disabled(is_on);
                            }
                        },
                        async {
                            loop {
                                short_press_button.until_click().await;
                                short_press_button.set_disabled(true);
                                long_press_button.set_disabled(true);
                                w.lock().await.send(WsMessage::Binary(to_allocvec(&MessageToEsp::ShortPressPowerButton).unwrap())).await.unwrap();
                            }
                        },
                        async {
                            loop {
                                long_press_button.until_click().await;
                                short_press_button.set_disabled(true);
                                long_press_button.set_disabled(true);
                                w.lock().await.send(WsMessage::Binary(to_allocvec(&MessageToEsp::LongPressPowerButton).unwrap())).await.unwrap();
                            }
                        }
                    )).await;
                },
                Br::new().render(),
                async {
                    let reset_button = Button::new();

                    join((
                        reset_button.render("Press reset button".render()),
                        async {
                            let mut stream = message_to_web_stream
                                .clone()
                                .filter_map(|(_, message)| Box::pin(async move {
                                    match message {
                                        MessageToWeb::ResetButtonStatus(is_on) => Some(is_on),
                                        _ => None
                                    }
                                }));
                            while let Some(is_on) = stream.next().await {
                                reset_button.set_disabled(is_on);
                            }
                        },
                        async {
                            loop {
                                reset_button.until_click().await;
                                reset_button.set_disabled(true);
                                w.lock().await.send(WsMessage::Binary(to_allocvec(&MessageToEsp::ShortPressResetButton).unwrap())).await.unwrap();
                            }
                        }
                    )).await;
                },
            )).await;
        }
        Err(e) => {
            error!(format!("{e:#?}"));
            format!("Error connecting web socket: {e}").render().await;
        }
    }
}
