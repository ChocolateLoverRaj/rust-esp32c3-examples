#![no_std]
#![no_main]

use core::{mem, ops::DerefMut};

use embassy_executor::Spawner;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, mutex::Mutex, signal::Signal};
use esp_backtrace as _;
use esp_hal::{
    gpio::{Input, InputConfig, Level, Output, OutputConfig, Pull},
    interrupt::software::SoftwareInterruptControl,
    timer::timg::TimerGroup,
    usb_serial_jtag::UsbSerialJtag,
};
use esp_println as _;
use futures::future::join3;

esp_bootloader_esp_idf::esp_app_desc!();

#[esp_rtos::main]
async fn main(spawner: Spawner) {
    let _ = spawner;

    let peripherals = esp_hal::init(Default::default());

    // Needed for esp_rtos
    let timg0 = TimerGroup::new(peripherals.TIMG0);
    let software_interrupt = SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    esp_rtos::start(timg0.timer0, software_interrupt.software_interrupt0);

    defmt::info!("Press t to toggle LED!");

    let mut led = Output::new(peripherals.GPIO8, Level::Low, OutputConfig::default());
    let mut button = Input::new(
        peripherals.GPIO9,
        InputConfig::default().with_pull(Pull::Down),
    );

    // We would use an AtomicUsize but it's not letting us do atomic swap
    let toggle_count = Mutex::<CriticalSectionRawMutex, _>::new(0_usize);
    let toggle_signal = Signal::<CriticalSectionRawMutex, ()>::new();

    join3(
        async {
            loop {
                button.wait_for_falling_edge().await;
                *toggle_count.lock().await += 1;
                toggle_signal.signal(());
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
                if input == b't' {
                    *toggle_count.lock().await += 1;
                    toggle_signal.signal(());
                }
            }
        },
        async {
            loop {
                toggle_signal.wait().await;
                let toggles = mem::take(toggle_count.lock().await.deref_mut());
                defmt::debug!("Processed {} toggles", toggles);
                if !toggles.is_multiple_of(2) {
                    let level = led.output_level();
                    let new_level = !level;
                    led.set_level(new_level);
                }
            }
        },
    )
    .await;
}
