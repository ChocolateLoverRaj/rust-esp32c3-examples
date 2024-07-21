use std::time::Duration;

use esp32_nimble::BLEDevice;
use futures::{select, FutureExt};
use log::info;
use smart_power_button_common::WakeupReason;

use crate::{power_io::PowerIo, value_channel::ValueReceiver, watch_power::Power};

/// Scans for certain Bluetooth devices and turns on the power button when they show up
pub async fn bluetooth_wake(
    mut power_io: PowerIo,
    mut wakeup_devices: ValueReceiver<Vec<[u8; 6]>>,
) {
    let ble_device = BLEDevice::take();
    let ble_scan = ble_device.get_scan();
    let mut scan_and_wake = async || {
        let device = loop {
            let wakeup_devices_now = wakeup_devices.get();
            if wakeup_devices_now.is_empty() {
                info!("No bluetooth wakeup devices. Not scanning");
                wakeup_devices.until_change().await;
            } else {
                info!("Scanning for these bluetooth devices: {wakeup_devices_now:?}");
                let scan_future = async {
                    loop {
                        match ble_scan
                            .find_device(i32::MAX, |device| {
                                wakeup_devices_now.contains(&device.addr().val())
                            })
                            .await
                        {
                            Ok(Some(device)) => break device,
                            Ok(None) => {
                                log::info!("Timed out finding a wake device. Will start again.");
                            }
                            Err(e) => {
                                log::error!("Error finding a wake device: {e:#?}");
                            }
                        }
                    }
                };
                let devices_change_future = wakeup_devices.until_change();
                select! {
                    device = scan_future.fuse() => {
                        break device
                    },
                    _ = devices_change_future.fuse() => {}
                };
            }
        };
        log::info!("Detected wake device: {device:#?}. Waking...");
        *power_io.wakeup_reason.lock().await = Some(WakeupReason::Bluetooth(device.addr().val()));
        power_io.power_button.short_press().await;
    };

    loop {
        match power_io.power_rx.get() {
            Some(Power::Off) | Some(Power::Suspend) => {
                log::info!("Scanning for Bluetooth devices...",);
                let scan_and_wake_future = scan_and_wake();
                let computer_turned_on_future = async {
                    loop {
                        power_io.power_rx.until_change().await;
                        if let Some(Power::On) = power_io.power_rx.get() {
                            break;
                        }
                    }
                };
                tokio::select! {
                    _ = scan_and_wake_future => {
                        log::info!("Computer was woken up. Cooldown until BLE scanning will start again.");
                        // Wait some time for the computer to actually turn on
                        tokio::time::sleep(Duration::from_secs(10)).await;
                    },
                    _ = computer_turned_on_future => {
                        log::info!("Computer is on. No longer scannign for Bluetooth devices.");
                    }
                }
            }
            _ => {
                power_io.power_rx.until_change().await;
            }
        }
    }
}
