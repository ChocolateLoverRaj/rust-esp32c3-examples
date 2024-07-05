use crate::gpio_pins_vec::GpioPinsVecExt;
use crate::power_io::PowerIo;
use crate::run_server::run_server;
use crate::wifi_loop::WifiLoop;
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::hal::peripherals::Peripherals;
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use esp_idf_svc::sys::esp;
use esp_idf_svc::timer::EspTaskTimerService;
use esp_idf_svc::wifi::{AsyncWifi, EspWifi};
use log::info;
use tokio::join;

mod button;
mod gpio_pins_vec;
mod handle_request;
mod http_content_type;
mod power_io;
mod run_server;
mod serve_static;
mod serve_websocket;
mod value_channel;
mod watch_input;
mod wifi_loop;

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
        timer.clone(),
    )?;

    let mut pins = peripherals
        .pins
        .into_indexable()
        .into_iter()
        .map(|pin| pin.into())
        .collect();
    let (power_io_future, power_io) = PowerIo::new(&mut pins)?;
    info!("Starting async run loop");
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async move {
            let mut wifi_loop = WifiLoop::new(wifi);
            wifi_loop.configure().await?;
            wifi_loop.initial_connect().await?;

            info!("Preparing to launch echo server...");
            tokio::spawn(run_server(wifi_loop.get_ip_info()?, power_io));

            info!("Entering main Wi-Fi run loop...");
            let _ = join!(power_io_future, wifi_loop.stay_connected());
            Ok::<(), anyhow::Error>(())
        })?;

    Ok(())
}

type Error = Box<dyn std::error::Error + Send + Sync + 'static>;
