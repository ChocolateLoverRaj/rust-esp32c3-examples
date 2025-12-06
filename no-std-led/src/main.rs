#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, mutex::Mutex};
use esp_backtrace as _;
use esp_hal::{
    gpio::{Input, InputConfig, Level, Output, OutputConfig, Pull},
    interrupt::software::SoftwareInterruptControl,
    timer::timg::TimerGroup,
    usb_serial_jtag::UsbSerialJtag,
};
use esp_println as _;
use futures::future::join;

esp_bootloader_esp_idf::esp_app_desc!();

#[esp_rtos::main]
async fn main(spawner: Spawner) {
    let _ = spawner;

    let peripherals = esp_hal::init(Default::default());

    // Needed for esp_rtos
    let timg0 = TimerGroup::new(peripherals.TIMG0);
    let software_interrupt = SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    esp_rtos::start(timg0.timer0, software_interrupt.software_interrupt0);

    defmt::info!(
        "Push the button, or press t to toggle LED, or press y/n to turn the LED on or off!"
    );

    let led = Mutex::<CriticalSectionRawMutex, _>::new(Output::new(
        peripherals.GPIO8,
        Level::Low,
        OutputConfig::default(),
    ));
    let mut button = Input::new(
        peripherals.GPIO9,
        InputConfig::default().with_pull(Pull::Down),
    );

    join(
        async {
            loop {
                button.wait_for_falling_edge().await;
                led.lock().await.toggle();
            }
        },
        async {
            let mut usb_serial = UsbSerialJtag::new(peripherals.USB_DEVICE).into_async();
            loop {
                let mut buffer = [Default::default(); 1];
                let len = embedded_io_async::Read::read(&mut usb_serial, &mut buffer)
                    .await
                    .unwrap();
                let input = buffer[..len][0];
                match input {
                    b't' => led.lock().await.toggle(),
                    b'y' => led.lock().await.set_low(),
                    b'n' => led.lock().await.set_high(),
                    _ => {}
                }
            }
        },
    )
    .await;
}
