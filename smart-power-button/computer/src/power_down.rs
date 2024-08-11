use std::time::Duration;

use crate::{
    config::{
        APP_TO_OPEN, IGNORE_TV_POWER_STATE, SHOULD_CONTROL_SOUND_SYSTEM, SHOULD_CONTROL_TV,
        SHOULD_SWITCH_SOUND_OUTPUT, TV_IP_ADDRESS,
    },
    samsung::Samsung,
    sound_system::SoundSystem,
    toggle_game_mode::toggle_game_mode,
    tv_data::{get_tv_data, save_tv_data},
};
use anyhow::Context;
use tokio::{time::sleep, try_join};

pub async fn power_down() -> anyhow::Result<()> {
    let mut tv_data = get_tv_data()
        .await
        .context("Error getting TV data")?
        .unwrap_or_default();
    println!("Read TV Data: {:#?}", tv_data);
    if tv_data.is_on || IGNORE_TV_POWER_STATE {
        println!("Turning off TV");
        let sound_system_future = async {
            if SHOULD_CONTROL_SOUND_SYSTEM {
                println!("Turning off sound system");
                SoundSystem::open().await?.turn_off().await?;
            }
            Ok::<_, anyhow::Error>(())
        };
        let tv_future = async {
            if SHOULD_CONTROL_TV {
                let mut remote = Samsung {
                    ip: TV_IP_ADDRESS.into(),
                    app_name: "Gaming Computer".into(),
                    token: tv_data.token.clone(),
                };
                toggle_game_mode(&mut remote).await?;
                sleep(Duration::from_secs_f64(1.0)).await;
                // Move all the way left
                for _ in 0..11 {
                    remote.send_key("KEY_LEFT").await?;
                    sleep(Duration::from_secs_f64(0.15)).await;
                }
                if SHOULD_SWITCH_SOUND_OUTPUT {
                    sleep(Duration::from_secs_f64(0.15)).await;
                    // Switch the sound output from Sound System to TV
                    remote.send_key("KEY_UP").await?;
                    sleep(Duration::from_secs_f64(0.15)).await;
                    remote.send_key("KEY_RIGHT").await?;
                    sleep(Duration::from_secs_f64(0.15)).await;
                    remote.send_key("KEY_RIGHT").await?;
                    sleep(Duration::from_secs_f64(0.15)).await;
                    remote.send_key("KEY_ENTER").await?;
                    sleep(Duration::from_secs_f64(0.15)).await;
                    // Go back to home settings
                    remote.send_key("KEY_DOWN").await?;
                }
                println!("Opening app");
                remote.open_app(APP_TO_OPEN).await?;
                sleep(Duration::from_secs_f64({
                    // 5.5s wasn't enough when Netflix was opened as a "cold start"
                    8.5
                }))
                .await;
                remote.send_key("KEY_POWER").await?;

                tv_data.token = remote.token
            }
            Ok::<_, anyhow::Error>(())
        };
        try_join!(sound_system_future, tv_future)?;

        tv_data.is_on = false;
        println!("Saving TV Data: {:#?}", tv_data);
        save_tv_data(&tv_data).await?;
    }
    Ok(())
}
