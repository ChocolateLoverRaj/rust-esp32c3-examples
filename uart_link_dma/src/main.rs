#![no_std]
#![no_main]

use core::future::pending;

use defmt::{error, info, warn};
use embassy_executor::Spawner;
use embassy_futures::join::join;
use embassy_time::{Instant, TICK_HZ};
use embedded_io_async::Write;
use esp_backtrace as _;
use esp_hal::{
    dma::{CHUNK_SIZE, DmaRxStreamBuf},
    dma_circular_buffers,
    interrupt::software::SoftwareInterruptControl,
    timer::timg::TimerGroup,
    uart::{
        Config, DataBits, StopBits, Uart,
        uhci::{self, Uhci},
    },
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

    let baud_rate = 5_000_000;
    let (mut uhci_rx, mut uhci_tx) = Uhci::new(
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

    join(
        async {
            info!("continuously sending data");
            let mut n = 0_u8;
            let mut buffer = [Default::default(); 128];
            loop {
                for byte in &mut buffer {
                    *byte = n;
                    n = n.wrapping_add(1);
                }
                Write::write_all(&mut uhci_tx.uart_tx, &buffer)
                    .await
                    .unwrap();
            }
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
                    for byte in data {
                        let missed_bytes = byte.wrapping_sub(n);
                        n = byte.wrapping_add(1);
                        if missed_bytes > 0 {
                            warn!("missed {} bytes", missed_bytes);
                        }
                    }
                    let now = Instant::now();
                    info!(
                        "Read {} B / (1s / {} * {})",
                        data.len(),
                        TICK_HZ,
                        (now - before).as_ticks()
                    );
                    before = now;
                    let data_len = data.len();
                    transfer.consume(data_len);
                }
            }
        },
    )
    .await;
}
