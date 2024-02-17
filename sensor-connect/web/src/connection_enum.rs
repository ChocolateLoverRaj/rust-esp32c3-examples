use crate::{ble_connection::BleConnection, usb_connection::UsbConnection};

pub enum ConnectionEnum {
    Usb(UsbConnection),
    Ble(BleConnection),
}
