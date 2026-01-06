#![no_std]
#![no_main]

use defmt::{error, info, warn};
use embassy_executor::Spawner;
use embassy_futures::{
    join::join,
    select::{Either, select},
};
use embassy_time::{Duration, Instant, TICK_HZ, Timer};
use embedded_io_async::{Read, Write};
use esp_backtrace as _;
use esp_hal::{
    interrupt::software::SoftwareInterruptControl,
    timer::timg::TimerGroup,
    uart::{Config, DataBits, RxError, StopBits, Uart},
};
use esp_println as _;

esp_bootloader_esp_idf::esp_app_desc!();

#[esp_rtos::main]
async fn main(spawner: Spawner) {
    let _ = spawner;

    let peripherals = esp_hal::init(Default::default());

    // Needed for esp_rtos
    let timg0 = TimerGroup::new(peripherals.TIMG0);
    let software_interrupt = SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    esp_rtos::start(timg0.timer0, software_interrupt.software_interrupt0);

    let baud_rate = 1_000_000;
    let (mut uart_rx, mut uart_tx) = Uart::new(
        peripherals.UART1,
        Config::default()
            .with_baudrate(baud_rate)
            .with_data_bits(DataBits::_8)
            .with_stop_bits(StopBits::_1),
    )
    .unwrap()
    .with_rx(peripherals.GPIO20)
    .with_tx(peripherals.GPIO21)
    .into_async()
    .split();

    join(
        async {
            let buffer = [Default::default(); 128];
            info!("continuously sending data");
            loop {
                Write::write_all(&mut uart_tx, &buffer).await.unwrap();
            }
            // loop {
            //     Write::write(&mut uart_tx, b"Hello other microcontroller")
            //         .await
            //         .unwrap();
            //     Timer::after(Duration::from_secs(2)).await;
            // }
        },
        async {
            let mut buffer = [Default::default(); 128];
            loop {
                info!("waiting for stream to start");
                loop {
                    match Read::read(&mut uart_rx, &mut buffer).await {
                        Ok(_) | Err(RxError::FifoOverflowed) => break,
                        Err(e) => warn!("rx error: {}", e),
                    }
                }
                info!("stream started");
                let mut before = Instant::now();
                loop {
                    let timeout = Duration::from_nanos({
                        let ideal_time_s = (buffer.len() * 8) as f64
                            / (baud_rate as f64 * (1.0 + 8.0 + 1.0) / 8.0);
                        // Allow up to double the expected time
                        let timeout_s = ideal_time_s * 2.0;
                        (timeout_s * 1e9) as u64
                    });
                    match select(
                        Read::read_exact(&mut uart_rx, &mut buffer),
                        Timer::after(timeout),
                    )
                    .await
                    {
                        Either::First(result) => match result {
                            Ok(()) => {
                                let now = Instant::now();
                                info!(
                                    "received data ({} B / (1s / {} * {}))",
                                    buffer.len(),
                                    TICK_HZ,
                                    (now - before).as_ticks()
                                );
                                before = now;
                            }
                            Err(e) => {
                                error!("error receiving data: {}", e);
                                break;
                            }
                        },
                        Either::Second(()) => {
                            error!("stopped receiving data");
                        }
                    };
                }
            }
            // let mut buffer = [Default::default(); 32];
            // loop {
            //     let read_len = Read::read(&mut uart_rx, &mut buffer).await.unwrap();
            //     let received_data = &buffer[..read_len];
            //     info!(
            //         "Received data: {:#02X} {:?}",
            //         received_data,
            //         str::from_utf8(received_data).ok()
            //     );
            // }
        },
    )
    .await;
}
