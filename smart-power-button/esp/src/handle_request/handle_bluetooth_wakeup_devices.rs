use http_body_util::combinators::BoxBody;
use http_body_util::BodyExt;
use hyper::body::Bytes;
use hyper::{Method, Request, Response, StatusCode};
use postcard::{from_bytes, to_allocvec};
use tokio::sync::Mutex;

use crate::bluetooth_wakeup_devices::BluetoothWakeupDevices;
use crate::hyper_util::{empty, full};
use crate::value_channel::ValueReceiver;
use crate::Error;

pub async fn handle_bluetooth_wakeup_devices(
    req: Request<hyper::body::Incoming>,
    wakeup_devices_tx: &Mutex<BluetoothWakeupDevices>,
    wakeup_devices_rx: ValueReceiver<Vec<[u8; 6]>>,
) -> Result<Response<BoxBody<Bytes, hyper::Error>>, Error> {
    match *req.method() {
        Method::GET => {
            let response = Response::new(full(to_allocvec(&wakeup_devices_rx.get()).unwrap()));
            Ok(response)
        }
        Method::PUT => {
            let devices = from_bytes(&req.collect().await?.to_bytes())?;
            match wakeup_devices_tx.lock().await.set(devices).await {
                Ok(()) => {
                    let response = Response::new(empty());
                    Ok(response)
                }
                Err(e) => {
                    log::error!("Error saving wakeup devices: {e:#?}");
                    let mut response = Response::new(empty());
                    *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
                    Ok(response)
                }
            }
        }
        _ => {
            let mut response = Response::new(empty());
            *response.status_mut() = StatusCode::METHOD_NOT_ALLOWED;
            Ok(response)
        }
    }
}
