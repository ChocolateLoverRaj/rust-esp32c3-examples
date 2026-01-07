#![no_std]
#![no_main]

use defmt::{error, info, warn};
use embassy_executor::Spawner;
use embassy_futures::{
    join::join,
    select::{Either, select},
};
use embassy_time::{Duration, Instant, TICK_HZ, Timer};
use embedded_io_async::Read;
use esp_backtrace as _;
use esp_hal::{
    dma::{CHUNK_SIZE, DmaRxBuf, DmaRxStreamBuf, DmaTxBuf},
    dma_circular_buffers,
    interrupt::software::SoftwareInterruptControl,
    timer::timg::TimerGroup,
    uart::{
        Config, DataBits, RxError, StopBits, Uart,
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

    let baud_rate = 0_010_000;
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
    let (rx_buffer, rx_descriptors, tx_buffer, tx_descriptors) =
        dma_circular_buffers!(4 * CHUNK_SIZE);

    join(
        async {
            info!("continuously sending data");
            let mut tx_buf = DmaTxBuf::new(tx_descriptors, tx_buffer).unwrap();
            uhci_tx.apply_config(&uhci::TxConfig::default()).unwrap();
            tx_buf.set_length(tx_buf.capacity());
            let mut n = 0;
            loop {
                for byte in tx_buf.as_mut_slice() {
                    *byte = n;
                    n = n.wrapping_add(1);
                }
                let mut transfer = uhci_tx
                    .write(tx_buf)
                    .unwrap_or_else(|(e, _, _)| panic!("{e:?}"));
                transfer.wait_for_done().await;
                let (result, returned_uhci_tx, returned_tx_buf) = transfer.wait();
                result.unwrap();
                uhci_tx = returned_uhci_tx;
                tx_buf = returned_tx_buf;
            }
        },
        async {
            let rx_buffer_len = rx_buffer.len();
            let mut rx_buf = DmaRxStreamBuf::new(rx_descriptors, rx_buffer).unwrap();
            loop {
                uhci_rx
                    .apply_config(
                        &uhci::RxConfig::default().with_chunk_limit(rx_buffer_len.min(4095) as u16),
                    )
                    .unwrap();
                let mut before;
                loop {
                    let mut transfer = uhci_rx
                        .read(rx_buf)
                        .unwrap_or_else(|(e, _, _)| panic!("{e:?}"));
                    loop {
                        before = Instant::now();
                        Timer::after(Duration::from_nanos({
                            let wanted_bytes = CHUNK_SIZE;
                            let delay_s = (wanted_bytes * 8) as f64
                                / (baud_rate as f64 * (1.0 + 8.0 + 1.0) / 8.0);
                            (delay_s * 1e9) as u64
                        }))
                        .await;
                        if transfer.is_done() {
                            todo!("We were too slow to read from the DMA");
                        }
                        let available_bytes = transfer.available_bytes();
                        let now = Instant::now();
                        info!(
                            "Read {} B / (1s / {} * {})",
                            available_bytes,
                            TICK_HZ,
                            (now - before).as_ticks()
                        );
                        transfer.consume(available_bytes);
                    }
                }
            }
        },
    )
    .await;
}
