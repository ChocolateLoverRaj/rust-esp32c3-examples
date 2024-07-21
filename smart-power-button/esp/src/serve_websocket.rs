use crate::watch_power::Power;
use crate::{Error, PowerIo};
use futures::stream::FuturesUnordered;
use futures::{SinkExt, StreamExt};
use hyper_tungstenite::tungstenite::Message;
use hyper_tungstenite::HyperWebsocket;
use log::warn;
use postcard::to_allocvec;
use smart_power_button_common::{MessageToEsp, MessageToWeb, WakeupReason};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Handle a websocket connection.
pub async fn serve_websocket(websocket: HyperWebsocket, power_io: PowerIo) -> Result<(), Error> {
    let PowerIo {
        mut power_led_rx,
        mut hdd_led_rx,
        power_button,
        reset_button,
        wakeup_reason,
        power_rx,
    } = power_io;
    let websocket = websocket.await?;
    let (w, mut r) = websocket.split();
    let w = Arc::new(Mutex::new(w));

    let futures: Vec<Pin<Box<dyn Future<Output = anyhow::Result<()>> + Send>>> = vec![
        Box::pin({
            let w = w.clone();
            async move {
                loop {
                    w.lock()
                        .await
                        .send(Message::Binary(to_allocvec(
                            &MessageToWeb::PowerLedStatus(power_led_rx.get()),
                        )?))
                        .await?;
                    power_led_rx.until_change().await;
                }
            }
        }),
        Box::pin({
            let w = w.clone();
            async move {
                loop {
                    w.lock()
                        .await
                        .send(Message::Binary(to_allocvec(&MessageToWeb::HddLedStatus(
                            hdd_led_rx.get(),
                        ))?))
                        .await?;
                    hdd_led_rx.until_change().await;
                }
            }
        }),
        Box::pin({
            let w = w.clone();
            let power_button = power_button.clone();
            async move {
                loop {
                    w.lock()
                        .await
                        .send(Message::Binary(to_allocvec(
                            &MessageToWeb::PowerButtonStatus(power_button.is_pressed().await),
                        )?))
                        .await?;
                    power_button.until_change().await;
                }
            }
        }),
        Box::pin({
            let w = w.clone();
            let reset_button = reset_button.clone();
            async move {
                loop {
                    w.lock()
                        .await
                        .send(Message::Binary(to_allocvec(
                            &MessageToWeb::ResetButtonStatus(reset_button.is_pressed().await),
                        )?))
                        .await?;
                    reset_button.until_change().await;
                }
            }
        }),
        Box::pin({
            let power_button = power_button.clone();
            async move {
                while let Some(message) = r.next().await {
                    if let Message::Binary(msg) = message? {
                        match postcard::from_bytes::<MessageToEsp>(&msg) {
                            Ok(message) => match message {
                                MessageToEsp::ShortPressPowerButton(should_turn_on_tv) => {
                                    tokio::spawn({
                                        let power_button = power_button.clone();
                                        let wakeup_reason = wakeup_reason.clone();
                                        let power_rx = power_rx.clone();
                                        async move {
                                            match power_rx.get() {
                                                Some(Power::Off) | Some(Power::Suspend) => {
                                                    *wakeup_reason.lock().await =
                                                        Some(WakeupReason::Web(should_turn_on_tv));
                                                }
                                                _ => {}
                                            }
                                            power_button.short_press().await
                                        }
                                    });
                                }
                                MessageToEsp::LongPressPowerButton => {
                                    tokio::spawn({
                                        let power_button = power_button.clone();
                                        async move { power_button.long_press().await }
                                    });
                                }
                                MessageToEsp::ShortPressResetButton => {
                                    tokio::spawn({
                                        let reset_button = reset_button.clone();
                                        async move { reset_button.short_press().await }
                                    });
                                }
                            },
                            Err(e) => {
                                warn!("Error parsing message: {e:?}");
                            }
                        }
                    }
                }
                Ok(())
            }
        }),
    ];
    let mut iter = futures.into_iter().collect::<FuturesUnordered<_>>();
    while let Some(result) = iter.next().await {
        result?
    }
    Ok(())
}
