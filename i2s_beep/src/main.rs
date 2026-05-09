#![no_std]
#![no_main]

use defmt::info;
use embassy_executor::Spawner;
use esp_backtrace as _;
use esp_hal::{
    dma_circular_buffers_chunk_size,
    i2s::master::{Channels, Config, DataFormat, I2s},
    interrupt::software::SoftwareInterruptControl,
    time::Rate,
    timer::timg::TimerGroup,
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

    let i2s = I2s::new(
        peripherals.I2S0,
        peripherals.DMA_CH0,
        Config::default()
            .with_sample_rate(Rate::from_khz(96))
            .with_data_format(DataFormat::Data32Channel32)
            .with_channels(Channels::MONO),
    )
    .unwrap()
    .into_async();
    let (_rx_buffer, _rx_descriptors, tx_buffer, tx_descriptors) =
        dma_circular_buffers_chunk_size!(0, 4092 * 4, 4092);
    let tx = i2s
        .i2s_tx
        .with_bclk(peripherals.GPIO10)
        .with_dout(peripherals.GPIO20)
        .with_ws(peripherals.GPIO21)
        .build(tx_descriptors);
    let mut transfer = tx.write_dma_circular_async(tx_buffer).unwrap();
    let samples: [i32; _] = [
        0, 134841614, 269151070, 402398309, 534057466, 663608942, 790541457, 914354066, 1034558137,
        1150679280, 1262259217, 1368857594, 1470053716, 1565448207, 1654664589, 1737350766,
        1813180413, 1881854265, 1943101299, 1996679799, 2042378316, 2080016499, 2109445808,
        2130550097, 2143246079, 2147483647, 2143246079, 2130550097, 2109445808, 2080016499,
        2042378316, 1996679799, 1943101299, 1881854265, 1813180413, 1737350766, 1654664589,
        1565448207, 1470053716, 1368857594, 1262259217, 1150679280, 1034558137, 914354066,
        790541457, 663608942, 534057466, 402398309, 269151070, 134841614,
    ]
    .map(|n| n / 64);
    let mut iterator = samples
        .iter()
        .cycle()
        .copied()
        .flat_map(|n| n.to_le_bytes());
    loop {
        let bytes_pushed = transfer
            .push_with(|buffer| {
                buffer.fill_with(|| iterator.next().unwrap());
                buffer.len()
            })
            .await
            .unwrap();
        // info!("Bytes pushed: {}", bytes_pushed);
    }
}
