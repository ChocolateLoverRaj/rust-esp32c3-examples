use std::{
    thread::{sleep, spawn},
    time::Duration,
};

use esp_idf_hal::task::block_on;
use futures::channel::oneshot::{self};
use futures::stream::StreamExt;
use futures::Stream;
use futures::{channel::mpsc::channel, stream::unfold, SinkExt};

pub fn get_stdin_stream(
    poll_frequency: Duration,
) -> (
    std::pin::Pin<Box<dyn Stream<Item = u8> + std::marker::Send>>,
    oneshot::Sender<()>,
) {
    let (stop_tx, mut stop_rx) = oneshot::channel::<()>();
    let (mut tx, rx) = channel::<u8>(1);
    let _handle = spawn(move || {
        block_on(async {
            loop {
                let byte = unsafe { libc::getchar() };
                if byte != -1 {
                    tx.send(byte as u8).await.unwrap();
                }
                if stop_rx.try_recv().is_ok_and(|v| v.is_some()) {
                    break;
                };
                sleep(poll_frequency);
            }
        })
    });

    let stream = unfold(rx, move |mut rx| async move {
        let chunk = rx.next().await.unwrap();
        Some((chunk, rx))
    })
    .boxed();
    (stream, stop_tx)
}
