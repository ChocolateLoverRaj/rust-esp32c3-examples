use esp_idf_hal::gpio::{Gpio9, InterruptType, PinDriver, Pull};
use futures::stream::unfold;

use crate::{action::Action, interface::Interface};

pub struct ButtonInterface {}

impl ButtonInterface {
    pub fn new(
        button: Gpio9,
    ) -> (
        Self,
        impl futures::prelude::Stream<Item = crate::action::Action>,
    ) {
        let mut button = PinDriver::input(button).unwrap();
        button.set_pull(Pull::Down).unwrap();
        button.set_interrupt_type(InterruptType::PosEdge).unwrap();
        button.enable_interrupt().unwrap();

        let button_stream = unfold(button, |mut button| async {
            button.wait_for_rising_edge().await.unwrap();
            Some((Action::Toggle, button))
        });
        (Self {}, button_stream)
    }
}

impl Interface for ButtonInterface {
    fn notify_change(&mut self) {}

    fn stop(self) {}
}
