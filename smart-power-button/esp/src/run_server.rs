use std::sync::Arc;

use crate::bluetooth_wakeup_devices::BluetoothWakeupDevices;
use crate::handle_request::handle_request;
use crate::power_io::PowerIo;
use crate::value_channel::ValueReceiver;
use esp_idf_svc::ipv4::IpInfo;
use hyper::service::service_fn;
use hyper::Request;
use hyper_util::rt::TokioIo;
use log::info;
use tokio::net::TcpListener;
use tokio::sync::Mutex;

pub async fn run_server(
    ip_info: IpInfo,
    hostname: &str,
    power_io: PowerIo,
    bluetooth_wakeup_devices_tx: BluetoothWakeupDevices,
    bluetooth_wakeup_devices_rx: ValueReceiver<Vec<[u8; 6]>>,
) -> anyhow::Result<()> {
    let addr = "0.0.0.0:80";

    info!("Binding to {addr}...");
    let listener = TcpListener::bind(&addr).await?;

    let ip = ip_info.ip;
    info!("Server is listening at http://{ip} and http://{hostname}");

    let bluetooth_wakeup_devices_tx = Arc::new(Mutex::new(bluetooth_wakeup_devices_tx));
    loop {
        info!("Waiting for new connection on socket: {listener:?}");
        let (stream, _) = listener.accept().await?;

        let io = TokioIo::new(stream);
        let bluetooth_wakeup_devices_tx = bluetooth_wakeup_devices_tx.clone();
        let bluetooth_wakeup_devices_rx = bluetooth_wakeup_devices_rx.clone();
        let power_io = power_io.clone();
        tokio::spawn({
            async move {
                info!("Spawned handler!");
                if let Err(err) = hyper::server::conn::http1::Builder::new()
                    .keep_alive(true)
                    // `service_fn` converts our function in a `Service`
                    .serve_connection(
                        io,
                        service_fn({
                            move |req: Request<hyper::body::Incoming>| {
                                handle_request(
                                    req,
                                    power_io.clone(),
                                    bluetooth_wakeup_devices_tx.clone(),
                                    bluetooth_wakeup_devices_rx.clone(),
                                )
                            }
                        }),
                    )
                    .with_upgrades()
                    .await
                {
                    eprintln!("Error serving connection: {:?}", err);
                }
            }
        });
    }
}
