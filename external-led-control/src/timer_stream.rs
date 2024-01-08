use futures::{
    channel::mpsc::{channel, Sender},
    future::{select, Either},
    stream::unfold,
    Stream, StreamExt,
};
use std::pin::pin;
use std::time::Duration;

use crate::async_timeout::async_timeout;

pub fn create_timer_stream(
    duration: Duration,
) -> (
    std::pin::Pin<Box<dyn Stream<Item = ()> + std::marker::Send>>,
    Sender<()>,
) {
    let (tx, rx) = channel::<()>(1);
    let stream = unfold(rx, move |mut rx| async move {
        loop {
            if let Either::Right(_) = select(rx.next(), pin!(async_timeout(duration.clone()))).await
            {
                break;
            }
        }
        Some(((), rx))
    })
    .boxed();
    (stream, tx)
}
