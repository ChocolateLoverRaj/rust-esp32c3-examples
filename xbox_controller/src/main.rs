#![no_std]
#![no_main]

use defmt::{error, info};
use embassy_executor::Spawner;
use embassy_futures::join::join;
use embassy_time::{Duration, Timer};
use esp_backtrace as _;
use esp_hal::{
    clock::CpuClock,
    efuse::{self},
    interrupt::software::SoftwareInterruptControl,
    rng::{Trng, TrngSource},
    timer::timg::TimerGroup,
};
use esp_println as _;
use esp_radio::ble::controller::BleConnector;
use trouble_host::{
    Address, Host, HostResources,
    gatt::GattClient,
    prelude::{
        AddrKind, BdAddr, Characteristic, ConnectConfig, ConnectionEvent, DefaultPacketPool,
        ExternalController, ScanConfig, Uuid,
    },
};

esp_bootloader_esp_idf::esp_app_desc!();

/// Max number of connections
const CONNECTIONS_MAX: usize = 1;

/// Max number of L2CAP channels.
const L2CAP_CHANNELS_MAX: usize = 3; // Signal + att + CoC

#[esp_rtos::main]
async fn main(spawner: Spawner) {
    let _ = spawner;

    let peripherals = esp_hal::init(esp_hal::Config::default().with_cpu_clock(CpuClock::max()));
    esp_alloc::heap_allocator!(size: 72 * 1024);

    // Needed for esp_rtos
    let timg0 = TimerGroup::new(peripherals.TIMG0);
    let software_interrupt = SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    esp_rtos::start(timg0.timer0, software_interrupt.software_interrupt0);

    info!("Hello from a Rust no_std environment with esp_rtos (basically embassy for ESP32).");

    let _trng_source = TrngSource::new(peripherals.RNG, peripherals.ADC1);
    let mut trng = Trng::try_new().unwrap(); // Ok when there's a TrngSource accessible

    let connector = BleConnector::new(peripherals.BT, Default::default()).unwrap();
    let controller = ExternalController::<_, 20>::new(connector);

    // Using a fixed "random" address can be useful for testing. In real scenarios, one would
    // use e.g. the MAC 6 byte array as the address (how to get that varies by the platform).
    let our_address = Address::random(
        esp_hal::efuse::interface_mac_address(efuse::InterfaceMacAddress::Bluetooth)
            .as_bytes()
            .try_into()
            .unwrap(),
    );
    info!("Our address = {:?}", our_address);

    let mut resources =
        HostResources::<DefaultPacketPool, CONNECTIONS_MAX, L2CAP_CHANNELS_MAX>::new();
    let stack = trouble_host::new(controller, &mut resources)
        .set_random_address(our_address)
        .set_random_generator_seed(&mut trng);
    stack.set_io_capabilities(trouble_host::IoCapabilities::NoInputNoOutput);
    let Host {
        mut central,
        mut runner,
        ..
    } = stack.build();

    let config = ConnectConfig {
        connect_params: Default::default(),
        scan_config: ScanConfig {
            filter_accept_list: &[(
                // 5C:BA:37:1D:74:5C
                AddrKind::PUBLIC,
                &BdAddr::new([0x5C, 0x74, 0x1D, 0x37, 0xBA, 0x5C]),
            )],
            ..Default::default()
        },
    };

    info!("Scanning for peripheral...");
    let _ = join(runner.run(), async {
        info!("Connecting");
        let conn = central.connect(&config).await.unwrap();
        // conn.set_bondable(true).unwrap();
        info!("Connected. Requestin pairing...");
        conn.request_security().unwrap();
        loop {
            match conn.next().await {
                ConnectionEvent::PairingComplete { security_level, .. } => {
                    info!("Pairing complete: {:?}", security_level);
                    break;
                }
                ConnectionEvent::PairingFailed(err) => {
                    error!("Pairing failed: {:?}", err);
                    break;
                }
                ConnectionEvent::Disconnected { reason } => {
                    error!("Disconnected: {:?}", reason);
                    break;
                }
                ConnectionEvent::RequestConnectionParams(req) => {
                    info!("Accepting request to update conection params");
                    req.accept(None, &stack).await.unwrap()
                }
                _ => {}
            }
        }
        // loop {
        //     let event = conn.next().await;
        //     match event {
        //         ConnectionEvent::RequestConnectionParams(req) => {
        //             info!("connection params request AFTER security");
        //             req.accept(None, &stack).await.unwrap();
        //             break;
        //         }
        //         event => {
        //             info!("ConnectionEvent: {}", event);
        //         }
        //     }
        // }
        join(
            async {
                info!("Paired. Creating GATT client...");
                let client = GattClient::<_, DefaultPacketPool, 10>::new(&stack, &conn)
                    .await
                    .unwrap();

                let _ = join(client.task(), async {
                    info!("Getting HID service");
                    let hid_service = client
                        .services_by_uuid(&Uuid::new_short(0x1812))
                        .await
                        .unwrap();
                    let hid_service = hid_service.first().unwrap();

                    info!("Getting descriptor characteristic");
                    let descriptor_characteristic: Characteristic<[u8; 512]> = client
                        .characteristic_by_uuid(&hid_service, &Uuid::new_short(0x2A4B))
                        .await
                        .unwrap();
                    info!("Reading descriptor characteristic");
                    let mut data = [0_u8; 512];
                    let bytes_read = client
                        .read_characteristic_long(&descriptor_characteristic, &mut data)
                        .await
                        .unwrap();
                    let feature_report = &data[..bytes_read];
                    info!("feature report: {:X}", feature_report);

                    info!("Getting report characteristic");
                    let report_characteristic: Characteristic<[u8; 9]> = client
                        .characteristic_by_uuid(&hid_service, &Uuid::new_short(0x2A4D))
                        .await
                        .unwrap();

                    let services = client.services().await.unwrap();
                    info!("services: {}", services.len());
                    for (i, service) in services.iter().enumerate() {
                        info!("{} {}", i, service);
                    }

                    let characteristics = client.characteristics::<10>(&hid_service).await.unwrap();
                    info!(
                        "found {} characteristics in the HID service.",
                        characteristics.len(),
                    );
                    for (i, characteristic) in characteristics.iter().enumerate() {
                        characteristic.props;
                        info!("{} {}", i, characteristic.props);
                    }
                    let rumble_characteristic = &characteristics[4];

                    info!("Sending rumble");
                    client
                        .write_characteristic(
                            &rumble_characteristic,
                            &[
                                // 0x03,   // Report ID
                                0b1111, // Mask (Enable all 4 motors)
                                0, 0, 0, 25, 200, // Duration (200 * 10ms = 2 seconds)
                                0,   // Delay (0)
                                0,   // Loop (0)
                            ],
                        )
                        .await
                        .unwrap();
                    info!("Sent rumble");
                    info!("Subscribing to report characteristic");
                    let mut listener = client
                        .subscribe(&report_characteristic, false)
                        .await
                        .unwrap();
                    info!("Subscribed.");

                    // Timer::after(Duration::from_secs(3)).await;

                    loop {
                        let notification = listener.next().await;
                        info!("Got notification: {:X}", notification.as_ref());
                    }
                })
                .await;
            },
            async {
                loop {
                    let event = conn.next().await;
                    match event {
                        ConnectionEvent::RequestConnectionParams(req) => {
                            info!("connection params request AFTER security");
                            req.accept(None, &stack).await.unwrap();
                        }
                        event => {
                            info!("ConnectionEvent: {}", event);
                        }
                    }
                }
            },
        )
        .await;
    })
    .await;
}
