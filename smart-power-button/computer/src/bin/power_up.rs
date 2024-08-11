use smart_power_button_computer::power_up::power_up;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    power_up().await
}
