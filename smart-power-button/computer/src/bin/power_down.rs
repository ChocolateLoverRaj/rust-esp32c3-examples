use std::time::Duration;

use smart_power_button_computer::{
    config::{
        APP_TO_OPEN, IGNORE_TV_POWER_STATE, SHOULD_CONTROL_SOUND_SYSTEM, SHOULD_CONTROL_TV,
        SHOULD_SWITCH_SOUND_OUTPUT,
    },
    samsung::Samsung,
    sound_system::SoundSystem,
    toggle_game_mode::toggle_game_mode,
    tv_data::{get_tv_data, save_tv_data},
};
use tokio::{join, time::sleep};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut tv_data = get_tv_data().await?.unwrap_or_default();
    println!("Read TV Data: {:#?}", tv_data);
    if tv_data.is_on || IGNORE_TV_POWER_STATE {
        println!("Turning off TV");
        let sound_system_future = async {
            if SHOULD_CONTROL_SOUND_SYSTEM {
                SoundSystem::open().await?.turn_off().await?;
            }
            Ok::<_, anyhow::Error>(())
        };
        let tv_future = async {
            if SHOULD_CONTROL_TV {
                let mut remote = Samsung {
                    ip: "samsung.local".into(),
                    app_name: "Gaming Computer".into(),
                    token: tv_data.token.clone(),
                };
                toggle_game_mode(&mut remote).await?;
                remote.send_key("KEY_HOME").await?;
                sleep(Duration::from_secs_f64(1.0)).await;
                // Move all the way left
                for _ in 0..11 {
                    remote.send_key("KEY_LEFT").await?;
                    sleep(Duration::from_secs_f64(0.15)).await;
                }
                if SHOULD_SWITCH_SOUND_OUTPUT {
                    // Switch the sound output from Sound System to TV
                    remote.send_key("KEY_UP").await?;
                    remote.send_key("KEY_RIGHT").await?;
                    remote.send_key("KEY_RIGHT").await?;
                    remote.send_key("KEY_ENTER").await?;
                    // Go back to home settings
                    remote.send_key("KEY_DOWN").await?;
                }
                remote.open_app(APP_TO_OPEN).await?;
                sleep(Duration::from_secs_f64(5.0)).await;
                remote.send_key("KEY_POWER").await?;

                tv_data.token = remote.token
            }
            Ok::<_, anyhow::Error>(())
        };
        {
            let (r1, r2) = join!(sound_system_future, tv_future);
            r1?;
            r2?;
        }

        tv_data.is_on = false;
        println!("Saving TV Data: {:#?}", tv_data);
        save_tv_data(&tv_data).await?;
    }
    Ok(())
}
