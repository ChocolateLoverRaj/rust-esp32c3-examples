use std::time::Duration;

use futures::Future;
use tokio::time::timeout;

use crate::value_channel::{value_channel, ValueReceiver};

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Power {
    On,
    Suspend,
    Off,
}

pub fn watch_power(
    mut power_led_rx: ValueReceiver<bool>,
) -> (
    impl Future<Output = anyhow::Result<()>>,
    ValueReceiver<Option<Power>>,
) {
    let blink_duration = Duration::from_secs(2);
    let (tx, rx) = value_channel(None);
    (
        async move {
            loop {
                let is_on = power_led_rx.get();
                if is_on {
                    // It's either on or suspend
                    match timeout(blink_duration, power_led_rx.until_change()).await {
                        Ok(()) => {
                            // It's either off or suspend
                            match timeout(blink_duration, power_led_rx.until_change()).await {
                                Ok(()) => {
                                    // It's suspend
                                    tx.update_if_changed(Some(Power::Suspend)).await;
                                    log::info!("Detected Power: {:?}", Power::Suspend);
                                }
                                Err(_elapsed) => {
                                    // It's off
                                    tx.update_if_changed(Some(Power::Off)).await;
                                    log::info!("Detected Power: {:?}", Power::Off);
                                }
                            }
                        }
                        Err(_elapsed) => {
                            // It's on
                            tx.update_if_changed(Some(Power::On)).await;
                            log::info!("Detected Power: {:?}", Power::On);
                        }
                    }
                } else {
                    // It's either off or suspend
                    match timeout(blink_duration, power_led_rx.until_change()).await {
                        Ok(()) => {
                            // It's either on or suspend
                            match timeout(blink_duration, power_led_rx.until_change()).await {
                                Ok(()) => {
                                    // It's suspend
                                    tx.update_if_changed(Some(Power::Suspend)).await;
                                    log::info!("Detected Power: {:?}", Power::Suspend);
                                }
                                Err(_elapsed) => {
                                    // It's on
                                    tx.update_if_changed(Some(Power::On)).await;
                                    log::info!("Detected Power: {:?}", Power::On);
                                }
                            }
                        }
                        Err(_elapsed) => {
                            // It's off
                            tx.update_if_changed(Some(Power::Off)).await;
                            log::info!("Detected Power: {:?}", Power::Off);
                        }
                    }
                }
            }
        },
        rx,
    )
}
