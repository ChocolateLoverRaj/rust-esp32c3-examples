pub use esp32c3_hal as hal;
pub type BootButton = crate::hal::gpio::Gpio9<crate::hal::gpio::Input<crate::hal::gpio::PullDown>>;
pub const SOC_NAME: &str = "ESP32-C3";
