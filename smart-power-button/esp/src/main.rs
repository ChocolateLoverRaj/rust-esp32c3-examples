#![feature(async_closure)]
#![feature(iter_intersperse)]

use std::time::Duration;

use crate::gpio_pins_vec::GpioPinsVecExt;
use crate::power_io::PowerIo;
use crate::run_server::run_server;
use crate::wifi_loop::WifiLoop;
use bluetooth_wake::bluetooth_wake;
use bluetooth_wakeup_devices::BluetoothWakeupDevices;
use embedded_graphics::image::Image;
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::{prelude::Point, Drawable};
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::hal::gpio::{InterruptType, PinDriver, Pull};
use esp_idf_svc::hal::peripherals::Peripherals;
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use esp_idf_svc::sys::esp;
use esp_idf_svc::timer::EspTaskTimerService;
use esp_idf_svc::wifi::{AsyncWifi, EspWifi};
use log::info;
use ssd1306::mode::DisplayConfig;
use ssd1306::prelude::{Brightness, DisplayRotation};
use ssd1306::size::DisplaySize128x64;
use ssd1306::{I2CDisplayInterface, Ssd1306};
use tinytga::Tga;
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

    let mut power_led = PinDriver::output(pins[7].take().unwrap())?;
    let mut hdd_led = PinDriver::output(pins[10].take().unwrap())?;
    let mut power_button = PinDriver::input(pins[5].take().unwrap())?;
    power_button.set_pull(Pull::Down)?;
    power_button.set_interrupt_type(InterruptType::AnyEdge)?;
    power_button.enable_interrupt()?;
    let mut hdd_button = PinDriver::input(pins[6].take().unwrap())?;
    hdd_button.set_pull(Pull::Down)?;
    hdd_button.set_interrupt_type(InterruptType::AnyEdge)?;
    hdd_button.enable_interrupt()?;

    let i2c = esp_idf_svc::hal::i2c::I2cDriver::new(
        peripherals.i2c0,
        pins[8].take().unwrap(),
        pins[9].take().unwrap(),
        &Default::default(),
    )?;
    let interface = I2CDisplayInterface::new(i2c);
    let mut display = Ssd1306::new(interface, DisplaySize128x64, DisplayRotation::Rotate0)
        .into_buffered_graphics_mode();
    display.init().unwrap();

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
                    loop {
                        match wifi_loop.initial_connect().await {
                            Ok(_) => break,
                            Err(e) => {
                                log::error!("Error connecting to wifi: {e:#?}. Retrying...");
                            }
                        }
                    }
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

            let power_led_future = async {
                loop {
                    power_led.set_high().unwrap();
                    tokio::time::sleep(Duration::from_millis(500)).await;
                    power_led.set_low().unwrap();
                    tokio::time::sleep(Duration::from_millis(500)).await;
                }
            };
            let hdd_led_future = async {
                loop {
                    hdd_led.set_high().unwrap();
                    tokio::time::sleep(Duration::from_millis(1000)).await;
                    hdd_led.set_low().unwrap();
                    tokio::time::sleep(Duration::from_millis(500)).await;
                }
            };

            let power_button_future = async {
                loop {
                    power_button.wait_for_any_edge().await.unwrap();
                    println!("Power button: {:?}", power_button.get_level());
                }
            };
            let hdd_button_future = async {
                loop {
                    hdd_button.wait_for_any_edge().await.unwrap();
                    println!("HDD button: {:?}", hdd_button.get_level());
                }
            };

            let display_future = async {
                let data = include_bytes!("../rust.tga");
                let tga: Tga<BinaryColor> = Tga::from_slice(data).unwrap();
                let image0 = Image::new(&tga, Point::zero());
                image0.draw(&mut display).unwrap();
                let image1 = Image::new(&tga, Point::new(64, 0));
                image1.draw(&mut display).unwrap();
                display.set_brightness(Brightness::custom(1, 1)).unwrap();
                let mut invert = false;
                loop {
                    display.flush().unwrap();
                    tokio::time::sleep(Duration::from_secs_f64(5.0)).await;
                    invert = !invert;
                    display.set_invert(invert).unwrap();
                }
            };

            info!("Entering main Wi-Fi run loop...");
            let _ = join!(
                power_io_future,
                server_future,
                bluetooth_wake_future,
                power_led_future,
                hdd_led_future,
                power_button_future,
                hdd_button_future,
                display_future
            );
            Ok::<(), anyhow::Error>(())
        })?;

    Ok(())
}

type Error = Box<dyn std::error::Error + Send + Sync + 'static>;
