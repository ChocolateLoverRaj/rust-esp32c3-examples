use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum MessageToEsp {
    /// Used for things like powering on, triggering whatever the OS does when the power button is pressed, and waking up from suspend
    ShortPressPowerButton,
    /// Used to force turn off the computer
    LongPressPowerButton,
    /// Used to force restart the computer
    ShortPressResetButton,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum MessageToWeb {
    /// If the power LED is on
    PowerLedStatus(bool),
    /// If the HDD led is on. (It's called the HDD led, but it also turns on when an SSD is in use).
    HddLedStatus(bool),
    /// If the power button is pressed
    PowerButtonStatus(bool),
    /// If the reset button is pressed
    ResetButtonStatus(bool)
}
