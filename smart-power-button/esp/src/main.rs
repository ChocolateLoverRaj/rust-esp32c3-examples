#![feature(async_closure)]

mod value_channel;
mod http_content_type;
mod wifi_loop;
mod button;
mod watch_input;
mod serve_static;

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::hal::gpio::{Gpio21, Gpio8};
use esp_idf_svc::hal::prelude::*;
use esp_idf_svc::timer::EspTaskTimerService;
use esp_idf_svc::ipv4::IpInfo;
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use esp_idf_svc::sys::esp;
use esp_idf_svc::wifi::{AsyncWifi, EspWifi};
use futures::{SinkExt, StreamExt};
use futures::stream::{FuturesUnordered};
use http_body_util::combinators::BoxBody;
use http_body_util::{BodyExt};
use hyper::body::Bytes;
use hyper::{Request, Response};
use hyper_tungstenite::{hyper, HyperWebsocket};
use hyper_tungstenite::hyper::service::service_fn;
use hyper_tungstenite::tungstenite::Message;
use hyper_util::rt::TokioIo;
use include_dir::{Dir, include_dir};
use log::{error, info, warn};
use postcard::to_allocvec;
use tokio::join;
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use smart_power_button_common::{MessageToEsp, MessageToWeb};
use crate::button::Button;
use crate::serve_static::serve_static;
use crate::value_channel::{ValueReceiver};
use crate::watch_input::watch_input;
use crate::wifi_loop::WifiLoop;

const ASSETS: Dir = include_dir!("$CARGO_MANIFEST_DIR/../web/dist");

struct PowerIo {
    power_led_rx: ValueReceiver<bool>,
    hdd_led_rx: ValueReceiver<bool>,
    power_button: Button<Gpio8>,
    reset_button: Button<Gpio21>,
}

impl Clone for PowerIo {
    fn clone(&self) -> Self {
        Self {
            power_led_rx: self.power_led_rx.clone(),
            hdd_led_rx: self.hdd_led_rx.clone(),
            power_button: self.power_button.clone(),
            reset_button: self.reset_button.clone(),
        }
    }
}

fn main() -> anyhow::Result<()> {
    // It is necessary to call this function once. Otherwise, some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    info!("Setting up eventfd...");
    let config = esp_idf_svc::sys::esp_vfs_eventfd_config_t {
        max_fds: 5,
        ..Default::default()
    };
    esp! { unsafe { esp_idf_svc::sys::esp_vfs_eventfd_register(&config) } }?;

    info!("Setting up board...");
    let peripherals = Peripherals::take().unwrap();
    let sysloop = EspSystemEventLoop::take()?;
    let timer = EspTaskTimerService::new()?;
    let nvs = EspDefaultNvsPartition::take()?;

    info!("Initializing Wi-Fi...");
    let wifi = AsyncWifi::wrap(
        EspWifi::new(peripherals.modem, sysloop.clone(), Some(nvs))?,
        sysloop,
        timer.clone())?;

    let (power_led_future, power_led_rx) = watch_input(peripherals.pins.gpio9)?;
    let (hdd_led_future, hdd_led_rx) = watch_input(peripherals.pins.gpio0)?;
    let power_button = Button::new(peripherals.pins.gpio8)?;
    let reset_button = Button::new(peripherals.pins.gpio21)?;
    let power_io = PowerIo {
        power_led_rx,
        hdd_led_rx,
        power_button,
        reset_button,
    };
    info!("Starting async run loop");
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async move {
            let mut wifi_loop = WifiLoop::new(wifi);
            wifi_loop.configure().await?;
            wifi_loop.initial_connect().await?;

            info!("Preparing to launch echo server...");
            tokio::spawn(server(wifi_loop.get_ip_info()?, power_io));

            info!("Entering main Wi-Fi run loop...");
            let _ = join!(power_led_future, hdd_led_future, wifi_loop.stay_connected());
            Ok::<(), anyhow::Error>(())
        })?;

    Ok(())
}

async fn server(ip_info: IpInfo, power_io: PowerIo) -> anyhow::Result<()> {
    let addr = "0.0.0.0:80";

    info!("Binding to {addr}...");
    let listener = TcpListener::bind(&addr).await?;

    let ip = ip_info.ip;
    info!("Server is listening at http://{ip}");

    loop {
        info!("Waiting for new connection on socket: {listener:?}");
        let (stream, _) = listener.accept().await?;

        let io = TokioIo::new(stream);

        let power_io = power_io.clone();
        tokio::spawn({
            async move {
                info!("Spawned handler!");
                if let Err(err) = hyper::server::conn::http1::Builder::new()
                    .keep_alive(true)
                    // `service_fn` converts our function in a `Service`
                    .serve_connection(io, service_fn({
                        move |req: Request<hyper::body::Incoming>| {
                            handle_request(req, power_io.clone())
                        }
                    }))
                    .with_upgrades()
                    .await
                {
                    eprintln!("Error serving connection: {:?}", err);
                }
            }
        });
    }
}

type Error = Box<dyn std::error::Error + Send + Sync + 'static>;

async fn handle_request(mut req: Request<hyper::body::Incoming>, power_io: PowerIo) -> Result<Response<BoxBody<Bytes, hyper::Error>>, Error> {
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
        serve_static(req).await
    }
}

/// Handle a websocket connection.
async fn serve_websocket(websocket: HyperWebsocket, power_io: PowerIo) -> Result<(), Error> {
    let PowerIo {
        mut power_led_rx,
        mut hdd_led_rx,
        power_button,
        reset_button
    } = power_io;
    let websocket = websocket.await?;
    let (w, mut r) = websocket.split();
    let w = Arc::new(Mutex::new(w));

    let futures: Vec<Pin<Box<dyn Future<Output=anyhow::Result<()>> + Send>>> = vec![
        Box::pin({
            let w = w.clone();
            async move {
                loop {
                    w.lock().await.send(Message::Binary(to_allocvec(&MessageToWeb::PowerLedStatus(power_led_rx.get()))?)).await?;
                    power_led_rx.until_change().await;
                }
            }
        }),
        Box::pin({
            let w = w.clone();
            async move {
                loop {
                    w.lock().await.send(Message::Binary(to_allocvec(&MessageToWeb::HddLedStatus(hdd_led_rx.get()))?)).await?;
                    hdd_led_rx.until_change().await;
                }
            }
        }),
        Box::pin({
            let w = w.clone();
            let power_button = power_button.clone();
            async move {
                loop {
                    w.lock().await.send(Message::Binary(to_allocvec(&MessageToWeb::PowerButtonStatus(power_button.is_pressed().await))?)).await?;
                    power_button.until_change().await;
                }
            }
        }),
        Box::pin({
            let w = w.clone();
            let reset_button = reset_button.clone();
            async move {
                loop {
                    w.lock().await.send(Message::Binary(to_allocvec(&MessageToWeb::ResetButtonStatus(reset_button.is_pressed().await))?)).await?;
                    reset_button.until_change().await;
                }
            }
        }),
        Box::pin({
            let power_button = power_button.clone();
            async move {
                while let Some(message) = r.next().await {
                    match message? {
                        Message::Binary(msg) => {
                            match postcard::from_bytes::<MessageToEsp>(&msg) {
                                Ok(message) => match message {
                                    MessageToEsp::ShortPressPowerButton => {
                                        tokio::spawn({
                                            let power_button = power_button.clone();
                                            async move {
                                                power_button.short_press().await
                                            }
                                        });
                                    }
                                    MessageToEsp::LongPressPowerButton => {
                                        tokio::spawn({
                                            let power_button = power_button.clone();
                                            async move {
                                                power_button.long_press().await
                                            }
                                        });
                                    }
                                    MessageToEsp::ShortPressResetButton => {
                                        tokio::spawn({
                                            let reset_button = reset_button.clone();
                                            async move {
                                                reset_button.short_press().await
                                            }
                                        });
                                    }
                                },
                                Err(e) => {
                                    warn!("Error parsing message: {e:?}");
                                }
                            }
                        }
                        _ => {}
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
