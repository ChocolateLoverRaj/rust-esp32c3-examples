use std::time::Duration;

use smart_power_button_computer::{
    config::{
        APP_TO_OPEN, IGNORE_TV_POWER_STATE, SHOULD_CONTROL_SOUND_SYSTEM, SHOULD_CONTROL_TV,
        SHOULD_SWITCH_SOUND_OUTPUT,
    },
    power_down::power_down,
    samsung::Samsung,
    sound_system::SoundSystem,
    toggle_game_mode::toggle_game_mode,
    tv_data::{get_tv_data, save_tv_data},
};
use tokio::{join, time::sleep};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    power_down().await
}
