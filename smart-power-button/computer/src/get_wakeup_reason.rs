use crate::config::REMOTE_ADDRESS;
use postcard::from_bytes;
use reqwest::Client;
use smart_power_button_common::WakeupReason;

pub async fn get_wakeup_reason() -> anyhow::Result<Option<WakeupReason>> {
    Ok(from_bytes(
        &Client::new()
            .delete(format!("http://{REMOTE_ADDRESS}/wakeup_reason"))
            .send()
            .await?
            .error_for_status()?
            .bytes()
            .await?,
    )?)
}
