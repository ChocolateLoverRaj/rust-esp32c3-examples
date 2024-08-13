use anyhow::Context;
use chrono::Local;
use smart_power_button_computer::{
    power_down::power_down,
    power_up::power_up,
    systemd_integration::{ExternalDeviceManager, OffReason},
};
use zbus::Connection;
use zbus_systemd::systemd1::ManagerProxy;

struct Service {}

impl ExternalDeviceManager for Service {
    async fn turn_on(&mut self) -> anyhow::Result<()> {
        println!("Turning on,  {:?}", Local::now());
        power_up().await.context("Error turning on TV")?;
        println!("Turned on,  {:?}", Local::now());
        Ok(())
    }
    async fn turn_off(&mut self, reason: OffReason) -> anyhow::Result<()> {
        println!("Turning off, {:?}", Local::now());
        if let OffReason::Suspend = reason {
            println!("Restarting NetworkManager");
            let connection = Connection::system().await?;
            let manager = ManagerProxy::new(&connection).await?;
            manager
                .restart_unit("NetworkManager.service".into(), "replace".into())
                .await
                .context("Error restarting NetworkManager")?;
        }
        power_down().await.context("Error turning off TV")?;
        // sleep(Duration::from_secs(50)).await;
        println!("Turned off,  {:?}", Local::now());
        Ok(())
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    Service {}.zbus_integration().await
}
