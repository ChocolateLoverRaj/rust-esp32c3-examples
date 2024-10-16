use std::{
    ops::Add,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

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

use crate::{
    async_vl53l0x::{AsyncVL53L0x, NewWithGpio1Error},
    subscribable2::Subscribable2,
};

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

#[derive(Clone, Copy, Debug)]
pub struct DistanceData {
    pub distance: u16,
    pub time: SystemTime,
}

impl DistanceData {
    pub fn to_bytes(&self) -> [u8; 10] {
        let distance = self.distance.to_be_bytes();
        let time = u64::try_from(self.time.duration_since(UNIX_EPOCH).unwrap().as_nanos())
            .unwrap()
            .to_be_bytes();

        distance
            .into_iter()
            .chain(time.into_iter())
            .collect::<Vec<u8>>()
            .try_into()
            .unwrap()
    }

    pub fn from_bytes(bytes: [u8; 9]) -> Self {
        Self {
            distance: u16::from_be_bytes(bytes[0..2].try_into().unwrap()),
            time: SystemTime::UNIX_EPOCH.add(Duration::from_nanos(u64::from_be_bytes(
                bytes[2..].try_into().unwrap(),
            ))),
        }
    }
}

pub type DistanceSubscribable = Subscribable2<DistanceData>;

pub fn distance_loop(
    vl53l0x: Arc<Mutex<DistanceSensor>>,
) -> (impl Future<Output = ()>, DistanceSubscribable) {
    let (subscribable, mut rx) = Subscribable2::new();
    (
        {
            let mut subscribable = subscribable.clone();
            async move {
                // Wait for start
                rx.next().await.unwrap();
                log::warn!("Started loop");
                vl53l0x.lock().await.vl53l0x.start_continuous(0).unwrap();
                loop {
                    match select(
                        rx.next(),
                        Box::pin(async { vl53l0x.lock().await.read_range_mm_async().await }),
                    )
                    .await
                    {
                        Either::Left((option, other_future)) => {
                            option.unwrap();
                            let distance = other_future.await.unwrap();
                            subscribable.update(DistanceData {
                                distance,
                                time: SystemTime::now(),
                            });
                            vl53l0x.lock().await.vl53l0x.stop_continuous().unwrap();

                            // Wait for start again
                            rx.next().await.unwrap();
                            vl53l0x.lock().await.vl53l0x.start_continuous(0).unwrap();
                        }
                        Either::Right((result, _)) => {
                            let distance = result.unwrap();
                            subscribable.update(DistanceData {
                                distance,
                                time: SystemTime::now(),
                            });
                        }
                    }
                }
            }
        },
        subscribable,
    )
}
