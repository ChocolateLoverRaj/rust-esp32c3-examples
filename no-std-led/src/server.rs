use trouble_host::prelude::*;

// GATT Server definition
#[gatt_server]
pub struct Server {
    pub battery_service: BatteryService,
}

/// Battery service
#[gatt_service(uuid = service::BATTERY)]
pub struct BatteryService {
    /// Battery Level
    #[descriptor(uuid = descriptors::VALID_RANGE, read, value = [0, 100])]
    #[descriptor(uuid = descriptors::MEASUREMENT_DESCRIPTION, name = "hello", read, value = "Battery Level")]
    #[characteristic(uuid = characteristic::BATTERY_LEVEL, read, notify, value = 10)]
    pub level: u8,
    #[characteristic(uuid = "408813df-5dd4-1f87-ec11-cdb001100000", write, read, notify)]
    pub status: bool,
}
