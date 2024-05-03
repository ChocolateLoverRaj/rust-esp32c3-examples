use std::{
    ops::Add,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use async_channel::{Receiver, unbounded};

use esp_idf_hal::{
    gpio::{Gpio1, Gpio2, Gpio3},
    i2c::{I2cConfig, I2cDriver, I2cError, I2C0},
    prelude::*,
};
use futures::{
    future::{select, Either},
    lock::Mutex,
    Future, StreamExt,
};
use futures_signals::signal::{Signal, SignalExt};
use log::info;
use common::distance_data::DistanceData;

use crate::{
    async_vl53l0x::{AsyncVL53L0x, NewWithGpio1Error},
    subscribable2::Subscribable2,
};
use crate::subscribable3::Subscribable3;

pub type DistanceSensor = AsyncVL53L0x<'static, I2cDriver<'static>, Gpio1>;

pub fn get_vl53l0x(
    sda: Gpio2,
    scl: Gpio3,
    i2c: I2C0,
    gpio1: Gpio1,
) -> Result<DistanceSensor, NewWithGpio1Error<I2cError>> {
    let config = I2cConfig::new().baudrate(1000.kHz().into());
    let i2c = I2cDriver::new(i2c, sda, scl, &config).unwrap();
    let mut async_vl53l0x = AsyncVL53L0x::new_with_gpio1(i2c, gpio1)?;
    async_vl53l0x
        .vl53l0x
        .set_measurement_timing_budget(20_000)
        .unwrap();
    Ok(async_vl53l0x)
}

pub fn distance_loop(
    distance_sensor: Arc<Mutex<DistanceSensor>>,
    subscribable: Arc<Subscribable3>
) -> (impl Future<Output = ()>, Receiver<DistanceData>) {
    let (tx, rx) = unbounded();
    (
        async move {
            let mut stream = subscribable.signal().to_stream();
            loop {
                let yes = stream.next().await.unwrap();
                if yes {
                    info!("Starting continuous distance measurement");
                    distance_sensor.lock().await.vl53l0x.start_continuous(0).unwrap();
                    loop {
                        let distance = distance_sensor.lock().await.read_range_mm_async().await.unwrap();
                        info!("(Continuously) got distance: {}mm", distance);
                        tx.send(DistanceData {
                            distance,
                            time: SystemTime::now(),
                        }).await.unwrap();
                        if !subscribable.get() {
                            break;
                        }
                    }
                    info!("Stopping continuous distance measurement");
                    distance_sensor.lock().await.vl53l0x.stop_continuous().unwrap()
                }
            }
        },
        rx,
    )
}
