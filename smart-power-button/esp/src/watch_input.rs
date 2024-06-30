use std::future::Future;
use esp_idf_svc::hal::gpio::{InputPin, InterruptType, OutputPin, PinDriver, Pull};
use log::info;
use crate::value_channel::{value_channel, ValueReceiver};

pub fn watch_input<T: InputPin + OutputPin>(pin: T) -> anyhow::Result<(impl Future<Output=anyhow::Result<()>>, ValueReceiver<bool>)> {
    let mut pin = PinDriver::input(pin)?;
    pin.set_pull(Pull::Down).unwrap();
    pin.set_interrupt_type(InterruptType::AnyEdge).unwrap();
    pin.enable_interrupt().unwrap();
    let (tx, rx) = value_channel(pin.is_low());
    Ok((async move {
        loop {
            pin.wait_for_any_edge().await?;
            info!("Pin {} is: {:?}", pin.pin(), pin.get_level());
            tx.update_if_changed(pin.is_low()).await;
        }
    }, rx))
}
