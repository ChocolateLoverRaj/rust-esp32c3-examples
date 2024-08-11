use std::time::Duration;

use tokio::time::sleep;

use crate::samsung::Samsung;

/// Toggles game mode, but doesn't exit the home menu after that
pub async fn toggle_game_mode(remote: &mut Samsung) -> anyhow::Result<()> {
    // This opens settings
    remote.send_key("KEY_MENU").await?;
    sleep(Duration::from_secs_f64(2.0)).await;
    // Go down to "General" settings tab
    for _ in 0..3 {
        remote.send_key("KEY_DOWN").await?;
        sleep(Duration::from_secs_f64(0.3)).await;
    }
    // Go right to the tab content
    remote.send_key("KEY_RIGHT").await?;
    // Go down to the "External Device Manager" button
    for _ in 0..2 {
        sleep(Duration::from_secs_f64(0.3)).await;
        remote.send_key("KEY_DOWN").await?;
    }
    sleep(Duration::from_secs_f64(0.3)).await;
    remote.send_key("KEY_ENTER").await?;
    sleep(Duration::from_secs_f64(0.3)).await;
    // Go down to the "Game Mode" button
    remote.send_key("KEY_DOWN").await?;
    sleep(Duration::from_secs_f64(0.5)).await;
    // Press the "Game Mode" button
    remote.send_key("KEY_ENTER").await?;
    sleep(Duration::from_secs_f64({
        // 2s sometimes isn't enough to show the change in Game Mode and can cause confusion about if game mode is on or off
        3.0
    }))
    .await;
    // Quickly exit the settings menu
    remote.send_key("KEY_HOME").await?;
    sleep(Duration::from_secs_f64(1.0)).await;
    Ok(())
}
