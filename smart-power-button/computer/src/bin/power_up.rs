use std::time::Duration;

use smart_power_button_common::WakeupReason;
use smart_power_button_computer::{
    config::{
        DEVICE_NAME, IGNORE_TV_POWER_STATE, SHOULD_CONTROL_SOUND_SYSTEM, SHOULD_CONTROL_TV,
        SHOULD_SWITCH_SOUND_OUTPUT, TV_MAC_ADDRESS,
    },
    get_wakeup_reason::get_wakeup_reason,
    power_up::power_up,
    samsung::Samsung,
    sound_system::SoundSystem,
    toggle_game_mode::toggle_game_mode,
    tv_data::{get_tv_data, save_tv_data},
};
use tokio::{join, time::sleep};
use wakey::WolPacket;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    power_up().await
}
