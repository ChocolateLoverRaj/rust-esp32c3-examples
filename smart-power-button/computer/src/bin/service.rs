use anyhow::Context;
use chrono::Local;
use smart_power_button_computer::{
    sound_system::SoundSystem, systemd_integration::ExternalDeviceManager,
};

struct Service {}

impl ExternalDeviceManager for Service {
    async fn turn_on(&mut self) -> anyhow::Result<()> {
        println!("Turning on,  {:?}", Local::now());
        SoundSystem::open()
            .await?
            .turn_on()
            .await
            .context("Error turning on sound system")?;
        println!("Turned on,   {:?}", Local::now());
        Ok(())
    }
    async fn turn_off(&mut self) -> anyhow::Result<()> {
        println!("Turning off, {:?}", Local::now());
        SoundSystem::open()
            .await?
            .turn_off()
            .await
            .context("Error turning off sound system")?;
        println!("Turned off,  {:?}", Local::now());
        // sleep(Duration::from_secs(2)).await;
        Ok(())
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    Service {}.zbus_integration().await
}
