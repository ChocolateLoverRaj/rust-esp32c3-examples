use futures::channel::oneshot;
use std::{
    thread::{sleep, spawn},
    time::Duration,
};

pub async fn async_timeout(duration: Duration) {
    let (tx, rx) = oneshot::channel::<()>();
    let _handle = spawn(move || {
        sleep(duration);
        let _ = tx.send(());
    });
    rx.await.unwrap();
}
