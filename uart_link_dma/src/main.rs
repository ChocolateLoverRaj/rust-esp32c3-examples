#![no_std]
#![no_main]

use core::future::pending;

use defmt::{error, info, warn};
use embassy_executor::Spawner;
use embassy_futures::join::join;
use embassy_time::{Duration, Instant, TICK_HZ};
use esp_backtrace as _;
use esp_hal::{
    dma::{CHUNK_SIZE, DmaRxStreamBuf},
    dma_circular_buffers, dma_loop_buffer,
    interrupt::software::SoftwareInterruptControl,
    timer::timg::TimerGroup,
    uart::{
        Config, DataBits, StopBits, Uart,
        uhci::{self, Uhci},
    },
};
use esp_println as _;
use heapless::Deque;

esp_bootloader_esp_idf::esp_app_desc!();

#[esp_rtos::main]
async fn main(spawner: Spawner) {
    let _ = spawner;

    let peripherals = esp_hal::init(Default::default());

    // Needed for esp_rtos
    let timg0 = TimerGroup::new(peripherals.TIMG0);
    let software_interrupt = SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    esp_rtos::start(timg0.timer0, software_interrupt.software_interrupt0);

    let baud_rate = 5_000_000;
    let (mut uhci_rx, uhci_tx) = Uhci::new(
        Uart::new(
            peripherals.UART1,
            Config::default()
                .with_baudrate(baud_rate)
                .with_data_bits(DataBits::_8)
                .with_stop_bits(StopBits::_1),
        )
        .unwrap()
        .with_rx(peripherals.GPIO20)
        .with_tx(peripherals.GPIO21),
        peripherals.UHCI0,
        peripherals.DMA_CH0,
    )
    .into_async()
    .split();
    let (rx_buffer, rx_descriptors, _tx_buffer, _tx_descriptors) =
        dma_circular_buffers!(4 * CHUNK_SIZE, 0);
    let mut samples = Deque::<f64, 500>::new();
    let mut average = 0f64;

    join(
        async {
            info!("continuously sending data");
            let mut n = 0_u8;
            let mut loop_buffer = dma_loop_buffer!(4095 / 256 * 256);
            for byte in loop_buffer.iter_mut() {
                *byte = n;
                n = n.wrapping_add(1);
            }
            let mut transfer = uhci_tx
                .write(loop_buffer)
                .unwrap_or_else(|(e, _, _)| panic!("{e:?}"));
            transfer.wait_for_done().await;
            let (result, _, _) = transfer.wait();
            result.unwrap();
            unreachable!();
        },
        async {
            let rx_buffer_len = rx_buffer.len();
            let rx_buf = DmaRxStreamBuf::new(rx_descriptors, rx_buffer).unwrap();

            uhci_rx
                .apply_config(
                    &uhci::RxConfig::default().with_chunk_limit(rx_buffer_len.min(4095) as u16),
                )
                .unwrap();
            let mut transfer = uhci_rx
                .read(rx_buf)
                .unwrap_or_else(|(e, _, _)| panic!("{e:?}"));
            let mut last_printed = Instant::now();
            let mut before = Instant::now();
            let mut n = 0;
            loop {
                let data = transfer.peek();
                if data.is_empty() {
                    if transfer.is_done() {
                        error!("transfer done");
                        pending::<()>().await;
                    }
                    embassy_futures::yield_now().await;
                } else {
                    let mut total_missed_bytes = 0_u64;
                    for byte in data {
                        let missed_bytes = byte.wrapping_sub(n);
                        n = byte.wrapping_add(1);
                        total_missed_bytes += missed_bytes as u64;
                    }
                    if total_missed_bytes > 0 {
                        warn!("missed {} bytes", total_missed_bytes);
                    }
                    if samples.is_full() {
                        average *= samples.len() as f64;
                        average -= samples.pop_back().unwrap();
                        average /= samples.len() as f64;
                    }

                    let now = Instant::now();
                    let sample = (data.len() as f64)
                        / (1.0 / TICK_HZ as f64 * (now - before).as_ticks() as f64);
                    before = now;

                    average *= samples.len() as f64;
                    samples.push_front(sample).unwrap();
                    average += sample;
                    average /= samples.len() as f64;
                    if last_printed.elapsed() >= Duration::from_secs(1) {
                        info!("received {} B/s", average);
                        last_printed = now;
                    }
                    let data_len = data.len();
                    transfer.consume(data_len);
                }
            }
        },
    )
    .await;
}
