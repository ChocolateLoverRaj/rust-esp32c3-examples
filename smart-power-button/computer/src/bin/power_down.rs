use smart_power_button_computer::power_down::power_down;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    power_down().await
}
