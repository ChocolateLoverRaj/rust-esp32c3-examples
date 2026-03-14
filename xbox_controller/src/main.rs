#![no_std]
#![no_main]

use defmt::{error, info};
use embassy_executor::Spawner;
use embassy_futures::join::join;
use embassy_time::{Duration, Timer, with_timeout};
use esp_backtrace as _;
use esp_hal::{
    clock::CpuClock,
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
        EventHandler, ExternalController, RequestedConnParams, ScanConfig, Uuid,
    },
};

esp_bootloader_esp_idf::esp_app_desc!();

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

    /// Max number of connections
    const CONNECTIONS_MAX: usize = 1;

    /// Max number of L2CAP channels.
    const L2CAP_CHANNELS_MAX: usize = 3; // Signal + att + CoC

    // Using a fixed "random" address can be useful for testing. In real scenarios, one would
    // use e.g. the MAC 6 byte array as the address (how to get that varies by the platform).
    let address: Address = Address::random([0xff, 0x8f, 0x1b, 0x05, 0xe4, 0xff]);
    info!("Our address = {:?}", address);

    let mut resources =
        HostResources::<DefaultPacketPool, CONNECTIONS_MAX, L2CAP_CHANNELS_MAX>::new();
    let stack = trouble_host::new(controller, &mut resources)
        .set_random_address(address)
        .set_random_generator_seed(&mut trng);
    stack.set_io_capabilities(trouble_host::IoCapabilities::NoInputNoOutput);
    let Host {
        mut central,
        mut runner,
        ..
    } = stack.build();

    let config = ConnectConfig {
        connect_params: RequestedConnParams {
            // 7.5ms = 7500μs (The minimum allowed by the BLE spec)
            min_connection_interval: Duration::from_micros(7500),

            // 10ms = 10000μs (A tight window for high responsiveness)
            max_connection_interval: Duration::from_millis(10),

            // 0 - Do not allow the controller to skip any connection events
            max_latency: 0,

            // Usually set to 0 or left at default for the stack to manage
            min_event_length: Duration::from_secs(0),
            max_event_length: Duration::from_secs(0),

            ..Default::default()
        },
        scan_config: ScanConfig {
            filter_accept_list: &[(
                // 5C:BA:37:1D:74:5C
                AddrKind::PUBLIC,
                &BdAddr::new([0x5C, 0x74, 0x1D, 0x37, 0xBA, 0x5C]),
            )],
            ..Default::default()
        },
    };

    struct MyHandler;
    impl EventHandler for MyHandler {}

    info!("Scanning for peripheral...");
    let _ =
        join(runner.run_with_handler(&MyHandler), async {
            info!("Connecting");
            let conn = central.connect(&config).await.unwrap();
            conn.set_bondable(true).unwrap();
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
                    ConnectionEvent::RequestConnectionParams(req) => req.accept(None, &stack).await.unwrap(),
                    _ => {}
                }
            }
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

            info!("Getting HID control point characteristic");
            let control_point_characteristic: Characteristic<u8> = client
                .characteristic_by_uuid(&hid_service, &Uuid::new_short(0x2A4C))
                .await
                .unwrap();
            info!("Writing to HID control point characteristic");
            client
                .write_characteristic_without_response(&control_point_characteristic, &[0x01])
                .await
                .unwrap();

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
            let report_characteristic: Characteristic<[u8; 512]> = client
                .characteristic_by_uuid(&hid_service, &Uuid::new_short(0x2A4D))
                .await
                .unwrap();

            info!("Subscribing to report characteristic");
            let mut listener = client
                .subscribe(&report_characteristic, false)
                .await
                .unwrap();
            info!("Subscribed.");

            join(async {
                loop {
                                match with_timeout(Duration::from_secs(10), listener.next()).await {
                                    Ok(data) => {
                                        info!("Got notification: {:X}", data.as_ref());
                                    }
                                    Err(_) => {
                                        info!("No notification. Reading manually to keep connection alive.");
                                        let mut buffer = [0_u8; 512];
                                        let bytes_read = client
                                            .read_characteristic_long(&report_characteristic, &mut buffer)
                                            .await
                                            .unwrap();
                                        let data = &data[..bytes_read];
                                        info!("polled data: {:X}", data);
                                    }
                                }
                            }
            }, async {
                // 1. Find the service
                let battery_service = client
                    .services_by_uuid(&Uuid::new_short(0x180F))
                    .await.unwrap();
                let battery_service = battery_service
                    .first()
                    .expect("Battery service not found");

                // 2. Find the characteristic
                let battery_level_char: Characteristic<u8> = client
                    .characteristic_by_uuid(&battery_service, &Uuid::new_short(0x2A19))
                    .await.unwrap();

                // 3. In your loop, read it periodically
                loop {
                let mut battery_buf = [0u8; 1];
                client.read_characteristic(&battery_level_char, &mut battery_buf).await.unwrap();
                info!("Battery Level: {}%", battery_buf[0]);
                Timer::after_secs(10).await;
                }
            }).await;

        })
        .await;
                },
                async {
                    loop {
                        let event = conn.next().await;
                        match event {
                            ConnectionEvent::RequestConnectionParams(req) => req.accept(None, &stack).await.unwrap(),
                            _ => {}
                        }
                    }
                },
            )
            .await;
        })
        .await;
}
