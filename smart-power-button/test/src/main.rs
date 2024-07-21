#![feature(iter_intersperse)]

use postcard::{from_bytes, to_allocvec};
use reqwest::{Client, Method};
use smart_power_button_common::WakeupReason;

const ADDRESS: &str = "192.168.1.253";

async fn set_bluetooth_wakeup_devices() {
    let ids = ["C8:3F:26:8D:4D:00", "5C:BA:37:1D:74:5C"]
        .iter()
        .map(|id| {
            let id: [_; 6] = {
                let mut bytes = id
                    .split(":")
                    .map(|hex| u8::from_str_radix(hex, 16).unwrap())
                    .collect::<Vec<_>>();
                bytes.reverse();
                bytes
            }
            .try_into()
            .unwrap();
            id
        })
        .collect::<Vec<_>>();
    Client::new()
        .put(format!("http://{ADDRESS}/bluetooth_wakeup_devices"))
        .body(to_allocvec(&ids).unwrap())
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
}

async fn get_bluetooth_wakeup_devices() {
    let ids: Vec<[u8; 6]> = from_bytes(
        &reqwest::get(format!("http://{ADDRESS}/bluetooth_wakeup_devices"))
            .await
            .unwrap()
            .bytes()
            .await
            .unwrap(),
    )
    .unwrap();
    let ids = ids
        .iter()
        .map(|id| {
            id.iter()
                // The string version's numbers need to be reversed
                .rev()
                .map(|id| format!("{id:02X}"))
                .intersperse(":".into())
                .collect::<String>()
        })
        .collect::<Vec<_>>();
    println!("{ids:#?}");
}

async fn get_wakeup_reason(delete: bool) {
    let reason: Option<WakeupReason> = from_bytes(
        &Client::new()
            .request(
                match delete {
                    true => Method::DELETE,
                    false => Method::GET,
                },
                format!("http://{ADDRESS}/wakeup_reason"),
            )
            .send()
            .await
            .unwrap()
            .error_for_status()
            .unwrap()
            .bytes()
            .await
            .unwrap(),
    )
    .unwrap();
    println!("Wakup reason: {reason:?}");
}

#[tokio::main]
async fn main() {
    set_bluetooth_wakeup_devices().await;
    get_bluetooth_wakeup_devices().await;
    get_wakeup_reason(true).await;
}
