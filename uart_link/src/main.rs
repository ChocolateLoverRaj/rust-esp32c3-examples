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
            info!("continuously sending data");
            let mut buffer = [Default::default(); 128];
            let mut n = u8::default();
            loop {
                for byte in &mut buffer {
                    *byte = n;
                    n = n.wrapping_add(1);
                }
                Write::write_all(&mut uart_tx, &buffer).await.unwrap();
            }
        },
        async {
            let mut buffer = [Default::default(); 128];
            loop {
                info!("waiting for stream to start");
                let mut last_byte = loop {
                    let mut buffer = Default::default();
                    match Read::read(&mut uart_rx, core::slice::from_mut(&mut buffer)).await {
                        Ok(_) | Err(RxError::FifoOverflowed) => break buffer,
                        Err(e) => warn!("rx error: {}", e),
                    }
                };
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
                                let mut total_bytes_missed = 0;
                                for byte in &buffer {
                                    let expected_byte = last_byte.wrapping_add(1);
                                    let bytes_missed = byte.wrapping_sub(expected_byte);
                                    total_bytes_missed += bytes_missed;
                                    last_byte = *byte;
                                }
                                if total_bytes_missed > 0 {
                                    warn!("missed {} bytes", total_bytes_missed);
                                }
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
                            break;
                        }
                    };
                }
            }
        },
    )
    .await;
}
