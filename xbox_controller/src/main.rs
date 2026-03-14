#![no_std]
#![no_main]

use defmt::{error, info};
use embassy_executor::Spawner;
use embassy_futures::join::join;
use embedded_storage::nor_flash::ReadNorFlash;
use esp_backtrace as _;
use esp_bootloader_esp_idf::partitions::{self, DataPartitionSubType, PartitionType};
use esp_hal::{
    clock::CpuClock,
    efuse::{self},
    interrupt::software::SoftwareInterruptControl,
    rng::{Trng, TrngSource},
    timer::timg::TimerGroup,
};
use esp_println as _;
use esp_radio::ble::controller::BleConnector;
use sequential_storage::{
    cache::NoCache,
    map::{MapConfig, MapStorage, PostcardValue},
};
use serde::{Deserialize, Serialize};
use trouble_host::{
    Address, BondInformation, Host, HostResources, Identity, IdentityResolvingKey, LongTermKey,
    gatt::GattClient,
    prelude::{
        AddrKind, BdAddr, Characteristic, ConnectConfig, ConnectionEvent, DefaultPacketPool,
        ExternalController, ScanConfig, SecurityLevel, Uuid,
    },
};

esp_bootloader_esp_idf::esp_app_desc!();

/// Max number of connections
const CONNECTIONS_MAX: usize = 1;

/// Max number of L2CAP channels.
const L2CAP_CHANNELS_MAX: usize = 3; // Signal + att + CoC

#[derive(Debug, Serialize, Deserialize)]
enum StoredSecurityLevel {
    NoEncryption,
    Encrypted,
    EncryptedAuthenticated,
}
impl From<SecurityLevel> for StoredSecurityLevel {
    fn from(value: SecurityLevel) -> Self {
        match value {
            SecurityLevel::NoEncryption => Self::NoEncryption,
            SecurityLevel::Encrypted => Self::Encrypted,
            SecurityLevel::EncryptedAuthenticated => Self::EncryptedAuthenticated,
        }
    }
}
impl From<StoredSecurityLevel> for SecurityLevel {
    fn from(value: StoredSecurityLevel) -> Self {
        match value {
            StoredSecurityLevel::NoEncryption => Self::NoEncryption,
            StoredSecurityLevel::Encrypted => Self::Encrypted,
            StoredSecurityLevel::EncryptedAuthenticated => Self::EncryptedAuthenticated,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct StoredBondInfo {
    ltk: u128,
    bd_addr: [u8; 6],
    irk: Option<u128>,
    is_bonded: bool,
    security_level: StoredSecurityLevel,
}
impl<'a> PostcardValue<'a> for StoredBondInfo {}
impl From<BondInformation> for StoredBondInfo {
    fn from(value: BondInformation) -> Self {
        Self {
            ltk: value.ltk.0,
            bd_addr: value.identity.bd_addr.into_inner(),
            irk: value.identity.irk.map(|irk| irk.0),
            is_bonded: value.is_bonded,
            security_level: value.security_level.into(),
        }
    }
}
impl From<StoredBondInfo> for BondInformation {
    fn from(value: StoredBondInfo) -> Self {
        Self {
            ltk: LongTermKey::new(value.ltk),
            identity: Identity {
                bd_addr: BdAddr(value.bd_addr),
                irk: value.irk.map(|irk| IdentityResolvingKey::new(irk)),
            },
            is_bonded: value.is_bonded,
            security_level: value.security_level.into(),
        }
    }
}

#[esp_rtos::main]
async fn main(spawner: Spawner) {
    let _ = spawner;

    let p = esp_hal::init(esp_hal::Config::default().with_cpu_clock(CpuClock::max()));
    esp_alloc::heap_allocator!(size: 72 * 1024);

    // Needed for esp_rtos
    let timg0 = TimerGroup::new(p.TIMG0);
    let software_interrupt = SoftwareInterruptControl::new(p.SW_INTERRUPT);
    esp_rtos::start(timg0.timer0, software_interrupt.software_interrupt0);

    // Using the `security` feature of trouble_host needs a random source
    let _trng_source = TrngSource::new(p.RNG, p.ADC1);
    let mut trng = Trng::try_new().unwrap(); // Ok when there's a TrngSource accessible

    let connector = BleConnector::new(p.BT, Default::default()).unwrap();
    let controller = ExternalController::<_, 20>::new(connector);

    // Use the NVS partition as the storage for the sequential_storage crate.
    let mut flash = esp_storage::FlashStorage::new(p.FLASH);
    let mut pt_mem = [0u8; partitions::PARTITION_TABLE_MAX_LEN];
    let partition_table = partitions::read_partition_table(&mut flash, &mut pt_mem).unwrap();
    let nvs = partition_table
        .find_partition(PartitionType::Data(DataPartitionSubType::Nvs))
        .unwrap()
        .unwrap();
    let nvs_storage = nvs.as_embedded_storage(&mut flash);
    let nvs_capacity = nvs_storage.capacity();
    let mut map = MapStorage::<(), _, _>::new(
        embassy_embedded_hal::adapter::BlockingAsync::new(nvs_storage),
        MapConfig::new(0..nvs_capacity as u32),
        NoCache::new(),
    );

    // Use the hardware bluetooth address
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
    let mut data_buffer = [0; 512];
    let stored_bond_information = map
        .fetch_item::<StoredBondInfo>(&mut data_buffer, &())
        .await
        .unwrap();
    if let Some(bond_information) = stored_bond_information {
        info!("Loading saved bond info");
        stack.add_bond_information(bond_information.into()).unwrap();
    }
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
        conn.set_bondable(true).unwrap();
        info!("Connected. Requestin pairing...");
        conn.request_security().unwrap();
        loop {
            match conn.next().await {
                ConnectionEvent::PairingComplete {
                    security_level,
                    bond,
                } => {
                    info!("Pairing complete: {:?}", security_level);
                    if let Some(bond) = bond {
                        info!("Storing bond");
                        map.store_item(&mut data_buffer, &(), &StoredBondInfo::from(bond))
                            .await
                            .unwrap();
                    }
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
                    // Note that if we don't respond to this request the Xbox controller will automatically disconnecting in ~60s.
                    info!("Accepting request to update connection params");
                    req.accept(None, &stack).await.unwrap()
                }
                _ => {}
            }
        }
        join(
            async {
                info!("Paired. Creating GATT client...");
                let client = GattClient::<_, DefaultPacketPool, 1>::new(&stack, &conn)
                    .await
                    .unwrap();

                let _ = join(client.task(), async {
                    info!("Getting HID service");
                    let hid_service = client
                        .services_by_uuid(&Uuid::new_short(0x1812))
                        .await
                        .unwrap();
                    let hid_service = hid_service.first().unwrap();

                    // The descriptor characteristic tells us about the data structure for inputs
                    // (button presses, etc) and outputs (rumble)
                    // Normally we would actually parse this but for this example we won't
                    info!("Getting descriptor characteristic");
                    let descriptor_characteristic: Characteristic<[u8; 512]> = client
                        .characteristic_by_uuid(&hid_service, &Uuid::new_short(0x2A4B))
                        .await
                        .unwrap();
                    info!("Reading descriptor characteristic");
                    let mut data = [0_u8; 512];
                    let bytes_read = client
                        .read_characteristic(&descriptor_characteristic, &mut data)
                        .await
                        .unwrap();
                    let feature_report = &data[..bytes_read];
                    info!("feature report: {:X}", feature_report);

                    // This example currently is hard-coded for a Xbox One S Controller (model 1708)
                    // It has one input report with id 0x1 and one output report with id 0x3
                    // If we wanted to be more generic we might need to support multiple input
                    // reports and multiple output reports
                    info!("Getting report characteristics");
                    let (input_report_characteristic, output_report_characteristic) = {
                        let mut input_report_characteristic = None;
                        let mut output_report_characteristic = None;
                        // For our controller we know it has exactly 5 characteristics in the HID service
                        let hid_characteristics =
                            client.characteristics::<5>(&hid_service).await.unwrap();
                        for characteristic in hid_characteristics {
                            if characteristic.uuid == Uuid::new_short(0x2A4D) {
                                // Read the Report Reference Descriptor
                                // The descriptor is two bytes
                                // The first byte is the report ID
                                // The second byte is if the characteristic is the report type
                                // 0x1 - Input
                                // 0x2 - Output
                                // 0x3 - Feature
                                let report_reference_descriptor = client
                                    .descriptor_by_uuid::<_, [u8; 2]>(
                                        &characteristic,
                                        &Uuid::new_short(0x2908),
                                    )
                                    .await
                                    .unwrap();
                                let mut buffer = [Default::default(); 2];
                                let bytes_read = client
                                    .read_descriptor(&report_reference_descriptor, &mut buffer)
                                    .await
                                    .unwrap();
                                let descriptor_value =
                                    <[u8; 2]>::try_from(&buffer[..bytes_read]).unwrap();
                                match descriptor_value {
                                    [0x1, 0x1] => {
                                        input_report_characteristic = Some(characteristic);
                                    }
                                    [0x3, 0x2] => {
                                        output_report_characteristic = Some(characteristic);
                                    }
                                    descriptor_value => {
                                        panic!(
                                            "Unexpected descriptor value: {descriptor_value:#X?}"
                                        )
                                    }
                                }
                            }
                        }
                        (
                            input_report_characteristic.unwrap(),
                            output_report_characteristic.unwrap(),
                        )
                    };

                    // let report_characteristic: Characteristic<[u8; 9]> = client
                    //     .characteristic_by_uuid(&hid_service, &Uuid::new_short(0x2A4D))
                    //     .await
                    //     .unwrap();

                    // let characteristics = client.characteristics::<10>(&hid_service).await.unwrap();
                    // info!(
                    //     "found {} characteristics in the HID service.",
                    //     characteristics.len(),
                    // );
                    // for (i, characteristic) in characteristics.iter().enumerate() {
                    //     info!(
                    //         "{} {} {} {} {}",
                    //         i,
                    //         characteristic.uuid,
                    //         characteristic.handle,
                    //         characteristic.cccd_handle,
                    //         characteristic.props
                    //     );
                    //     if let Ok(descriptor) = client
                    //         .descriptor_by_uuid::<_, [u8; 2]>(
                    //             characteristic,
                    //             &Uuid::new_short(0x2908),
                    //         )
                    //         .await
                    //     {
                    //         let mut buffer = [0; 2];
                    //         let bytes_read = client
                    //             .read_descriptor(&descriptor, &mut buffer)
                    //             .await
                    //             .unwrap();
                    //         let descriptor = &buffer[..bytes_read];
                    //         info!("Descriptor: {:X}", descriptor);
                    //     }
                    // }
                    // let rumble_characteristic = &characteristics[4];

                    info!("Subscribing to report characteristic");
                    let mut listener = client
                        .subscribe(&input_report_characteristic, false)
                        .await
                        .unwrap();
                    info!("Subscribed.");

                    // The Xbox controller has 4 haptic motors in the following order
                    // Left trigger
                    // Right trigger
                    // Heavy rumble (which is located on the left side of the controller)
                    // Small rumble (which is located on the right side of the controller)

                    // When we send a rumble command we can select which motors we are talking to
                    info!("Sending rumble");
                    client
                        .write_characteristic(
                            &output_report_characteristic,
                            &[
                                // The first byte is a bit mask of which motors we want to update
                                // Here we will update all 4 motors
                                0b1111,
                                // Next are values (as a percentage, with 100 being max) for the intensity
                                // for the 4 motors
                                // We will only enable the small motor, at 10%
                                0, 0, 0, 10,
                                // This is the duration. The number is multiplied by 10ms
                                // We set it to 2s
                                200,
                                // This is the delay. The number is multiplied by 10ms
                                // Set to no delay
                                0,
                                // This is the number of times we want to repeat it, with 0 being
                                // just play the vibration once, and 1 being repeat it once after
                                // The first one
                                // Set to 0
                                0,
                            ],
                        )
                        .await
                        .unwrap();
                    info!("Sent rumble");

                    // Watch for inputs and print them
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
                            // Note that if we don't respond to this request the Xbox controller will automatically disconnecting in ~60s.
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
