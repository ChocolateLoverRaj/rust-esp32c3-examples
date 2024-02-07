use std::{
    borrow::BorrowMut,
    ops::Add,
    sync::{Arc, Mutex},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use esp_idf_hal::gpio::{AnyIOPin, Gpio5, Gpio8, IOPin, Input, InterruptType, PinDriver, Pull};
use futures::{
    future::{select, Either},
    Future, StreamExt,
};

use crate::subscribable2::Subscribable2;

pub type ReceiverPin = PinDriver<'static, AnyIOPin, Input>;
pub fn configure_and_get_receiver_pin(gpio5: Gpio5) -> ReceiverPin {
    let mut receiver_pin = PinDriver::input(gpio5.downgrade()).unwrap();
    receiver_pin.set_pull(Pull::Down).unwrap();
    receiver_pin
        .set_interrupt_type(InterruptType::AnyEdge)
        .unwrap();
    receiver_pin.enable_interrupt().unwrap();
    receiver_pin
}

pub fn is_receiving_light(receiver_pin: &mut PinDriver<'static, AnyIOPin, Input>) -> bool {
    receiver_pin.is_low()
}

#[derive(Clone, Copy, Debug)]
pub struct IrData {
    pub is_receiving_light: bool,
    pub time: SystemTime,
}

impl IrData {
    pub fn to_bytes(&self) -> [u8; 9] {
        let bool = [u8::from(self.is_receiving_light)];
        let time = u64::try_from(self.time.duration_since(UNIX_EPOCH).unwrap().as_nanos())
            .unwrap()
            .to_be_bytes();

        bool.into_iter()
            .chain(time.into_iter())
            .collect::<Vec<u8>>()
            .try_into()
            .unwrap()
    }

    pub fn from_bytes(bytes: [u8; 9]) -> Self {
        Self {
            is_receiving_light: bytes[0] != 0,
            time: SystemTime::UNIX_EPOCH.add(Duration::from_nanos(u64::from_be_bytes(
                bytes[1..].try_into().unwrap(),
            ))),
        }
    }
}

pub type IrSubscribable = Subscribable2<IrData>;

pub fn ir_loop(
    receiver_pin: Arc<Mutex<ReceiverPin>>,
    gpio8: Gpio8,
) -> (impl Future<Output = ()>, IrSubscribable) {
    let (subscribable, mut rx) = Subscribable2::new();
    (
        {
            let mut subscribable = subscribable.clone();
            async move {
                // Initialize Pin 8 as an output to drive the LED
                let mut led_pin = PinDriver::output(gpio8).unwrap();

                let mut previous = None::<bool>;
                rx.next().await.unwrap();
                log::warn!("Starting ir loop");
                loop {
                    let mut receiver_pin = receiver_pin.lock().unwrap();
                    let is_receiving_light = is_receiving_light(receiver_pin.borrow_mut());
                    if is_receiving_light {
                        led_pin.set_low().unwrap();
                    } else {
                        led_pin.set_high().unwrap();
                    }
                    if previous.map_or(true, |previous| is_receiving_light != previous) {
                        subscribable.update(IrData {
                            is_receiving_light,
                            time: SystemTime::now(),
                        });
                    }
                    previous = Some(is_receiving_light);
                    let r = select(rx.next(), Box::pin(receiver_pin.wait_for_any_edge())).await;
                    match r {
                        Either::Left((option, _)) => {
                            option.unwrap();
                            rx.next().await.unwrap();
                        }
                        Either::Right((result, _)) => {
                            result.unwrap();
                        }
                    }
                }
            }
        },
        subscribable,
    )
}
