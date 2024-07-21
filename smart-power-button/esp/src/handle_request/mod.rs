use std::ops::Deref;

use crate::bluetooth_wakeup_devices::BluetoothWakeupDevices;
use crate::power_io::PowerIo;
use crate::serve_websocket::serve_websocket;
use crate::value_channel::ValueReceiver;
use crate::Error;
use handle_bluetooth_wakeup_devices::handle_bluetooth_wakeup_devices;
use handle_wakeup_reason::handle_wakeup_reason;
use http_body_util::combinators::BoxBody;
use http_body_util::BodyExt;
use hyper::body::Bytes;
use hyper::{Request, Response};
use log::error;
use serve_static::serve_static;
use tokio::sync::Mutex;

mod handle_bluetooth_wakeup_devices;
mod handle_wakeup_reason;
mod serve_static;

pub async fn handle_request<B: Deref<Target = Mutex<BluetoothWakeupDevices>>>(
    mut req: Request<hyper::body::Incoming>,
    power_io: PowerIo,
    bluetooth_wakeup_devices_tx: B,
    bluetooth_wakeup_devices_rx: ValueReceiver<Vec<[u8; 6]>>,
) -> Result<Response<BoxBody<Bytes, hyper::Error>>, Error> {
    // Check if the request is a websocket upgrade request.
    if hyper_tungstenite::is_upgrade_request(&req) {
        let (response, websocket) = hyper_tungstenite::upgrade(&mut req, None)?;

        // Spawn a task to handle the websocket connection.
        tokio::spawn(async move {
            if let Err(e) = serve_websocket(websocket, power_io).await {
                error!("Error in websocket connection: {e}");
            }
        });

        // Return the response so the spawned future can continue.
        Ok(response.map(|a| a.map_err(|never| match never {}).boxed()))
    } else {
        match req.uri().path() {
            "/wakeup_reason" => handle_wakeup_reason(req, power_io).await,
            "/bluetooth_wakeup_devices" => handle_bluetooth_wakeup_devices(
                req,
                &bluetooth_wakeup_devices_tx,
                bluetooth_wakeup_devices_rx,
            ).await,
            _ => serve_static(req).await,
        }
    }
}
