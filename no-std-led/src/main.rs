#![no_std]
#![no_main]

mod server;

use core::pin::pin;

use defmt::{info, warn};
use embassy_executor::Spawner;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, mutex::Mutex};
use embassy_time::Timer;
use esp_backtrace as _;
use esp_hal::{
    gpio::{Input, InputConfig, Level, Output, OutputConfig, Pull},
    interrupt::software::SoftwareInterruptControl,
    timer::timg::TimerGroup,
    usb_serial_jtag::UsbSerialJtag,
};
use esp_println as _;
use esp_radio::{self, ble::controller::BleConnector};
use futures::future::{join, join3, select};
use trouble_host::{Address, Host, HostResources, prelude::*};

use crate::server::Server;

esp_bootloader_esp_idf::esp_app_desc!();

#[esp_rtos::main]
async fn main(spawner: Spawner) {
    let _ = spawner;

    let peripherals = esp_hal::init(Default::default());

    // Needed for esp_rtos
    let timg0 = TimerGroup::new(peripherals.TIMG0);
    let software_interrupt = SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    esp_rtos::start(timg0.timer0, software_interrupt.software_interrupt0);

    esp_alloc::heap_allocator!(size: 72 * 1024);

    defmt::info!(
        "Push the button, or press t to toggle LED, or press y/n to turn the LED on or off!"
    );

    let led = Mutex::<CriticalSectionRawMutex, _>::new(Output::new(
        peripherals.GPIO8,
        Level::Low,
        OutputConfig::default(),
    ));

    join3(
        async {
            let mut button = Input::new(
                peripherals.GPIO9,
                InputConfig::default().with_pull(Pull::Down),
            );
            loop {
                button.wait_for_falling_edge().await;
                led.lock().await.toggle();
            }
        },
        async {
            let mut usb_serial = UsbSerialJtag::new(peripherals.USB_DEVICE).into_async();
            loop {
                let mut buffer = [Default::default(); 1];
                let len = embedded_io_async::Read::read(&mut usb_serial, &mut buffer)
                    .await
                    .unwrap();
                let input = buffer[..len][0];
                match input {
                    b't' => led.lock().await.toggle(),
                    b'y' => led.lock().await.set_low(),
                    b'n' => led.lock().await.set_high(),
                    _ => {}
                }
            }
        },
        async {
            let radio = esp_radio::init().unwrap();
            let bluetooth = peripherals.BT;
            let connector = BleConnector::new(&radio, bluetooth, Default::default()).unwrap();
            let controller = ExternalController::<_, 20>::new(connector);
            // Using a fixed "random" address can be useful for testing. In real scenarios, one would
            // use e.g. the MAC 6 byte array as the address (how to get that varies by the platform).
            let address = Address::random([0xff, 0x8f, 0x1a, 0x05, 0xe4, 0xff]);
            info!("Our address = {:?}", address);
            let mut resources: HostResources<DefaultPacketPool, 5, 5> = HostResources::new();
            let stack = trouble_host::new(controller, &mut resources).set_random_address(address);
            let Host {
                mut peripheral,
                mut runner,
                ..
            } = stack.build();
            info!("Starting advertising and GATT service");

            let server = Server::new_with_config(GapConfig::Peripheral(PeripheralConfig {
                name: "TrouBLE",
                appearance: &appearance::light_source::LED_LAMP,
            }))
            .unwrap();

            join(
                async {
                    loop {
                        runner.run().await.unwrap();
                    }
                },
                async {
                    loop {
                        match advertise("ESP32 LED", &mut peripheral, &server).await {
                            Ok(conn) => {
                                // set up tasks when the connection is established to a central, so they don't run when no one is connected.
                                let a = gatt_events_task(&server, &conn);
                                let b = custom_task(&server, &conn, &stack);
                                let a = pin!(a);
                                let b = pin!(b);
                                // run until any task ends (usually because the connection has been closed),
                                // then return to advertising state.
                                select(a, b).await;
                            }
                            Err(e) => {
                                let e = defmt::Debug2Format(&e);
                                panic!("[adv] error: {:?}", e);
                            }
                        }
                    }
                },
            )
            .await;
        },
    )
    .await;
}

/// Create an advertiser to use to connect to a BLE Central, and wait for it to connect.
async fn advertise<'values, 'server, C: Controller>(
    name: &'values str,
    peripheral: &mut Peripheral<'values, C, DefaultPacketPool>,
    server: &'server Server<'values>,
) -> Result<GattConnection<'values, 'server, DefaultPacketPool>, BleHostError<C::Error>> {
    let mut advertiser_data = [0; 31];
    let len = AdStructure::encode_slice(
        &[
            AdStructure::Flags(LE_GENERAL_DISCOVERABLE | BR_EDR_NOT_SUPPORTED),
            AdStructure::ServiceUuids16(&[[0x0f, 0x18]]),
            AdStructure::CompleteLocalName(name.as_bytes()),
        ],
        &mut advertiser_data[..],
    )?;
    let advertiser = peripheral
        .advertise(
            &Default::default(),
            Advertisement::ConnectableScannableUndirected {
                adv_data: &advertiser_data[..len],
                scan_data: &[],
            },
        )
        .await?;
    info!("[adv] advertising");
    let conn = advertiser.accept().await?.with_attribute_server(server)?;
    info!("[adv] connection established");
    Ok(conn)
}

/// Stream Events until the connection closes.
///
/// This function will handle the GATT events and process them.
/// This is how we interact with read and write requests.
async fn gatt_events_task<P: PacketPool>(
    server: &Server<'_>,
    conn: &GattConnection<'_, '_, P>,
) -> Result<(), Error> {
    let level = server.battery_service.level;
    let reason = loop {
        match conn.next().await {
            GattConnectionEvent::Disconnected { reason } => break reason,
            GattConnectionEvent::Gatt { event } => {
                match &event {
                    GattEvent::Read(event) => {
                        if event.handle() == level.handle {
                            let value = server.get(&level);
                            info!("[gatt] Read Event to Level Characteristic: {:?}", value);
                        }
                    }
                    GattEvent::Write(event) => {
                        if event.handle() == level.handle {
                            info!(
                                "[gatt] Write Event to Level Characteristic: {:?}",
                                event.data()
                            );
                        }
                    }
                    _ => {}
                };
                // This step is also performed at drop(), but writing it explicitly is necessary
                // in order to ensure reply is sent.
                match event.accept() {
                    Ok(reply) => reply.send().await,
                    Err(e) => warn!("[gatt] error sending response: {:?}", e),
                };
            }
            _ => {} // ignore other Gatt Connection Events
        }
    };
    info!("[gatt] disconnected: {:?}", reason);
    Ok(())
}

/// Example task to use the BLE notifier interface.
/// This task will notify the connected central of a counter value every 2 seconds.
/// It will also read the RSSI value every 2 seconds.
/// and will stop when the connection is closed by the central or an error occurs.
async fn custom_task<C: Controller, P: PacketPool>(
    server: &Server<'_>,
    conn: &GattConnection<'_, '_, P>,
    stack: &Stack<'_, C, P>,
) {
    let mut tick: u8 = 0;
    let level = server.battery_service.level;
    loop {
        tick = tick.wrapping_add(1);
        info!("[custom_task] notifying connection of tick {}", tick);
        if level.notify(conn, &tick).await.is_err() {
            info!("[custom_task] error notifying connection");
            break;
        };
        // read RSSI (Received Signal Strength Indicator) of the connection.
        if let Ok(rssi) = conn.raw().rssi(stack).await {
            info!("[custom_task] RSSI: {:?}", rssi);
        } else {
            info!("[custom_task] error getting RSSI");
            break;
        };
        Timer::after_secs(2).await;
    }
}
