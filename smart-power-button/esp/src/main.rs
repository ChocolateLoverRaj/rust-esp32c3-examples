#![feature(async_closure)]
#![feature(iter_intersperse)]

use crate::gpio_pins_vec::GpioPinsVecExt;
use crate::power_io::PowerIo;
use crate::run_server::run_server;
use crate::wifi_loop::WifiLoop;
use bluetooth_wake::bluetooth_wake;
use bluetooth_wakeup_devices::BluetoothWakeupDevices;
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::hal::peripherals::Peripherals;
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use esp_idf_svc::sys::esp;
use esp_idf_svc::timer::EspTaskTimerService;
use esp_idf_svc::wifi::{AsyncWifi, EspWifi};
use log::info;
use tokio::join;

mod bluetooth_wake;
mod bluetooth_wakeup_devices;
mod button;
mod gpio_pins_vec;
mod handle_request;
mod http_content_type;
mod hyper_util;
mod power_io;
mod run_server;
mod serve_websocket;
mod value_channel;
mod watch_input;
mod watch_power;
mod wifi_loop;

fn main() -> anyhow::Result<()> {
    // It is necessary to call this function once. Otherwise, some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    info!("Setting up eventfd...");
    let config = esp_idf_svc::sys::esp_vfs_eventfd_config_t { max_fds: 5 };
    esp! { unsafe { esp_idf_svc::sys::esp_vfs_eventfd_register(&config) } }?;

    info!("Setting up board...");
    let peripherals = Peripherals::take().unwrap();
    let sysloop = EspSystemEventLoop::take()?;
    let timer = EspTaskTimerService::new()?;
    let nvs = EspDefaultNvsPartition::take()?;

    info!("Initializing Wi-Fi...");
    let wifi = AsyncWifi::wrap(
        EspWifi::new(peripherals.modem, sysloop.clone(), Some(nvs.clone()))?,
        sysloop,
        timer.clone(),
    )?;

    let mut pins = peripherals
        .pins
        .into_indexable()
        .into_iter()
        .map(|pin| pin.into())
        .collect::<Vec<_>>();
    let (power_io_future, power_io) = PowerIo::new(&mut pins)?;
    let (bluetooth_wakeup_devices_tx, bluetooth_wakeup_devices_rx) =
        BluetoothWakeupDevices::new(nvs.clone())?;
    info!("Starting async run loop");
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async move {
            let mut wifi_loop = WifiLoop::new(wifi);

            info!("Preparing to launch server...");
            let server_future = {
                let power_io = power_io.clone();
                let bluetooth_wakeup_devices_rx = bluetooth_wakeup_devices_rx.clone();
                async move {
                    wifi_loop.configure().await.unwrap();
                    wifi_loop.initial_connect().await.unwrap();
                    let (ip_info, hostname) = wifi_loop.get_ip_info();
                    let _ = join!(
                        run_server(
                            ip_info,
                            &hostname,
                            power_io,
                            bluetooth_wakeup_devices_tx,
                            bluetooth_wakeup_devices_rx
                        ),
                        wifi_loop.stay_connected()
                    );
                }
            };

            let bluetooth_wake_future =
                bluetooth_wake(power_io.clone(), bluetooth_wakeup_devices_rx.clone());

            info!("Entering main Wi-Fi run loop...");
            let _ = join!(power_io_future, server_future, bluetooth_wake_future);
            Ok::<(), anyhow::Error>(())
        })?;

    Ok(())
}

type Error = Box<dyn std::error::Error + Send + Sync + 'static>;
