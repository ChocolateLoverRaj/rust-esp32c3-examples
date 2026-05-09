#![no_std]
#![no_main]

use defmt::{Debug2Format, info};
use embassy_executor::Spawner;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, mutex::Mutex};
use embassy_time::{Delay, Duration, Timer};
use embedded_hal::digital::OutputPin;
use esp_backtrace as _;
use esp_hal::{
    dma::{DmaRxBuf, DmaTxBuf},
    dma_buffers,
    gpio::{Level, Output, OutputConfig},
    interrupt::software::SoftwareInterruptControl,
    spi::master::{Config, Spi},
    time::Rate,
    timer::timg::TimerGroup,
};
use esp_println as _;
use esp_println::println;
use spi_sd_card::{
    Action, ActionResponse, ResetAndInit, format_command, prepare_command_0, process_command_0,
};

esp_bootloader_esp_idf::esp_app_desc!();

#[esp_rtos::main]
async fn main(spawner: Spawner) {
    let _ = spawner;

    let peripherals = esp_hal::init(Default::default());

    // Needed for esp_rtos
    let timg0 = TimerGroup::new(peripherals.TIMG0);
    let software_interrupt = SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    esp_rtos::start(timg0.timer0, software_interrupt.software_interrupt0);

    let (rx_buffer, rx_descriptors, tx_buffer, tx_descriptors) = dma_buffers!(1024);
    let dma_rx_buf = DmaRxBuf::new(rx_descriptors, rx_buffer).unwrap();
    let dma_tx_buf = DmaTxBuf::new(tx_descriptors, tx_buffer).unwrap();

    let mut spi_bus = Spi::new(
        peripherals.SPI2,
        Config::default().with_frequency(Rate::from_khz(400)),
    )
    .unwrap()
    .with_sck(peripherals.GPIO4)
    .with_mosi(peripherals.GPIO3)
    .with_miso(peripherals.GPIO2)
    .with_dma(peripherals.DMA_CH0)
    .with_buffers(dma_rx_buf, dma_tx_buf)
    .into_async();

    let mut cs = Output::new(peripherals.GPIO1, Level::High, OutputConfig::default());

    // // Send 74 clock cycles
    // // Rounded up to 10 bytes
    // spi_bus.write_async(&[0xFF; 10]).await.unwrap();

    // cs.set_low();

    // let mut buffer = [0xFF; 9];
    // prepare_command_0(&mut buffer);
    // spi_bus.transfer_in_place_async(&mut buffer).await.unwrap();
    // let r1 = process_command_0(&buffer);
    // info!("Buffer: {:X}", Debug2Format(&r1));

    let mut buffer = [0xFF; 100];
    let mut card = ResetAndInit::default();
    loop {
        let action_response = match card.action() {
            Action::TransferInPlace { len, prepare } => {
                let buffer = &mut buffer[..len];
                prepare(buffer);
                spi_bus.transfer_in_place_async(buffer).await.unwrap();
                ActionResponse::Transferred(buffer)
            }
            Action::SetCs(pin_state) => {
                cs.set_state(pin_state);
                ActionResponse::SetCs
            }
        };
        card = card.next(action_response).unwrap();
    }
}
