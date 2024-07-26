use std::time::Duration;

use tokio::time::sleep;

use crate::samsung::Samsung;

/// Toggles game mode, but doesn't exit the home menu after that
pub async fn toggle_game_mode(remote: &mut Samsung) -> anyhow::Result<()> {
    remote.send_key("KEY_MENU").await?;
    sleep(Duration::from_secs_f64(1.0)).await;
    for _ in 0..3 {
        remote.send_key("KEY_DOWN").await?;
        sleep(Duration::from_secs_f64(0.3)).await;
    }
    remote.send_key("KEY_RIGHT").await?;
    sleep(Duration::from_secs_f64(0.3)).await;
    remote.send_key("KEY_DOWN").await?;
    sleep(Duration::from_secs_f64(0.3)).await;
    remote.send_key("KEY_DOWN").await?;
    sleep(Duration::from_secs_f64(0.3)).await;
    remote.send_key("KEY_ENTER").await?;
    remote.send_key("KEY_DOWN").await?;
    sleep(Duration::from_secs_f64(0.5)).await;
    remote.send_key("KEY_ENTER").await?;
    sleep(Duration::from_secs_f64(2.0)).await;
    remote.send_key("KEY_HOME").await?;
    sleep(Duration::from_secs_f64(1.0)).await;
    Ok(())
}
