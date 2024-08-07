use std::time::{Duration, SystemTime};

use anyhow::Context;
use chrono::Local;
use smart_power_button_computer::{
    power_down::power_down, power_up::power_up, sound_system::SoundSystem,
    systemd_integration::ExternalDeviceManager,
};
use tokio::time::sleep;

struct Service {
    sound_system: SoundSystem,
}

impl ExternalDeviceManager for Service {
    async fn turn_on(&mut self) -> anyhow::Result<()> {
        println!("Turning on,  {:?}", Local::now());
        self.sound_system
            .turn_on()
            .await
            .context("Error turning on sound system")?;
        println!("Turned on,   {:?}", Local::now());
        Ok(())
    }
    async fn turn_off(&mut self) -> anyhow::Result<()> {
        println!("Turning off, {:?}", Local::now());
        self.sound_system
            .turn_off()
            .await
            .context("Error turning off sound system")?;
        println!("Turned off,  {:?}", Local::now());
        Ok(())
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    Service {
        sound_system: SoundSystem::open().await?,
    }
    .zbus_integration()
    .await
}
