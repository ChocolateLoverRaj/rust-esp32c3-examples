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
    dma::{DmaRxBuf, DmaTxBuf},
    dma_buffers,
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
    let (rx_buffer, rx_descriptors, tx_buffer, tx_descriptors) = dma_buffers!(4 * 4092);

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
            let mut rx_buf = DmaRxBuf::new(rx_descriptors, rx_buffer).unwrap();
            loop {
                info!("waiting for stream to start");
                let mut expected_n = {
                    let mut buffer = Default::default();
                    loop {
                        match Read::read(&mut uhci_rx.uart_rx, core::slice::from_mut(&mut buffer))
                            .await
                        {
                            Ok(_) | Err(RxError::FifoOverflowed) => break buffer,
                            Err(e) => warn!("rx error: {}", e),
                        }
                    }
                } + 1;

                info!("stream started");
                rx_buf.set_length(rx_buf.capacity());
                uhci_rx
                    .apply_config(
                        &uhci::RxConfig::default().with_chunk_limit(rx_buf.len().min(4095) as u16),
                    )
                    .unwrap();
                let mut before = Instant::now();
                let mut total_missed_bytes = 0_u64;
                loop {
                    let timeout = Duration::from_nanos({
                        let ideal_time_s = (rx_buf.len() * 8) as f64
                            / (baud_rate as f64 * (1.0 + 8.0 + 1.0) / 8.0);
                        // Allow up to double the expected time
                        let timeout_s = ideal_time_s * 2.0;
                        (timeout_s * 1e9) as u64
                    });
                    // FIXME: Data between transfers might get lost
                    let mut transfer = uhci_rx
                        .read(rx_buf)
                        .unwrap_or_else(|(e, _, _)| panic!("{e:?}"));
                    if total_missed_bytes > 0 {
                        // black_box(total_missed_bytes);
                        warn!("missed {} bytes", total_missed_bytes);
                        total_missed_bytes = 0;
                    } else {
                        info!("missed 0 bytes");
                    }
                    match select(transfer.wait_for_done(), Timer::after(timeout)).await {
                        Either::First(()) => {
                            let (result, returned_uhci_rx, returned_rx_buf) = transfer.wait();
                            uhci_rx = returned_uhci_rx;
                            rx_buf = returned_rx_buf;
                            match result {
                                Ok(()) => {
                                    let now = Instant::now();
                                    for byte in rx_buf.received_data().flatten() {
                                        let missed_bytes = byte.wrapping_sub(expected_n);
                                        total_missed_bytes += missed_bytes as u64;
                                        expected_n = *byte + 1;
                                    }
                                    // info!(
                                    //     "received data ({} B / (1s / {} * {}))",
                                    //     rx_buf.number_of_received_bytes(),
                                    //     TICK_HZ,
                                    //     (now - before).as_ticks()
                                    // );
                                    before = now;
                                    // Timer::after_nanos(1).await;
                                }
                                Err(e) => {
                                    error!("error receiving data: {}", e);
                                    break;
                                }
                            }
                        }
                        Either::Second(()) => {
                            let (returned_uhci_rx, returned_rx_buf) = transfer.cancel();
                            uhci_rx = returned_uhci_rx;
                            rx_buf = returned_rx_buf;
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
