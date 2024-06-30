use std::future::Future;
use dotenvy_macro::dotenv;
use esp_idf_svc::hal::gpio::{AnyIOPin, AnyOutputPin};
use tokio::join;
use crate::button::Button;
use crate::string_to_error::{ResultWrapErrorMessageExt, StringError};
use crate::value_channel::ValueReceiver;
use crate::watch_input::watch_input;

pub const POWER_LED_PIN: &str = dotenv!("POWER_LED_PIN");
pub const HDD_LED_PIN: &str = dotenv!("HDD_LED_PIN");
pub const POWER_BUTTON_PIN: &str = dotenv!("POWER_BUTTON_PIN");
pub const RESET_BUTTON_PIN: &str = dotenv!("RESET_BUTTON_PIN");

pub struct PowerIo {
    pub power_led_rx: ValueReceiver<bool>,
    pub hdd_led_rx: ValueReceiver<bool>,
    pub power_button: Button<AnyOutputPin>,
    pub reset_button: Button<AnyOutputPin>,
}

impl Clone for PowerIo {
    fn clone(&self) -> Self {
        Self {
            power_led_rx: self.power_led_rx.clone(),
            hdd_led_rx: self.hdd_led_rx.clone(),
            power_button: self.power_button.clone(),
            reset_button: self.reset_button.clone(),
        }
    }
}

fn take_pin(pins: &mut Vec<Option<AnyIOPin>>, pin: &str, pin_name: &str) -> anyhow::Result<AnyIOPin> {
    let pin_number = pin.parse::<usize>().wrap_err_message(format!("Error parsing pin number: {pin:?} for {pin_name}"))?;
    let pin = pins
        .get_mut(pin_number)
        .ok_or::<StringError>(format!("Invalid pin number: {pin_number:?} for {pin_name}").into())?
        .take()
        .ok_or::<StringError>(format!("Pin number {pin_number:?} for {pin_name} is already in use").into())?;
    Ok(pin)
}

impl PowerIo {
    pub fn new(pins: &mut Vec<Option<AnyIOPin>>) -> anyhow::Result<(impl Future<Output=()> + Sized, Self)> {
        let (power_led_future, power_led_rx) = watch_input(take_pin(pins, POWER_LED_PIN, "Power LED")?)?;
        let (hdd_led_future, hdd_led_rx) = watch_input(take_pin(pins, HDD_LED_PIN, "HDD LED")?)?;
        let power_button = Button::new(take_pin(pins, POWER_BUTTON_PIN, "Power Button")?.into())?;
        let reset_button = Button::new(take_pin(pins, RESET_BUTTON_PIN, "Reset Button")?.into())?;
        let power_io = Self {
            power_led_rx,
            hdd_led_rx,
            power_button,
            reset_button,
        };
        Ok((async {
            // TODO: Error handling
            let _ = join!(power_led_future, hdd_led_future);
        }, power_io))
    }
}
