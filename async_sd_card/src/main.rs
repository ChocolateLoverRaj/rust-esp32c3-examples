#![no_std]
#![no_main]

use defmt::info;
use embassy_executor::Spawner;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, mutex::Mutex};
use embassy_time::{Delay, Duration, Timer};
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
use heapless::String;
use pure_fat::{Bpb, DirEntryParser, ParseEntryOutput};
use pure_mbr::GenericMbr;
use spi_sd_card::{BLOCK_SIZE, Disk, EmbassySharedSpiBus, SpiSdCard};
use zerocopy::transmute;

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

    let spi_bus = Mutex::<CriticalSectionRawMutex, _>::new(
        Spi::new(peripherals.SPI2, Config::default())
            .unwrap()
            .with_sck(peripherals.GPIO6)
            .with_mosi(peripherals.GPIO7)
            .with_miso(peripherals.GPIO2)
            .with_dma(peripherals.DMA_CH0)
            .with_buffers(dma_rx_buf, dma_tx_buf)
            .into_async(),
    );

    let cs = Output::new(peripherals.GPIO1, Level::High, OutputConfig::default());

    let mut sd_card = SpiSdCard::new(
        EmbassySharedSpiBus::new(&spi_bus),
        cs,
        Delay,
        Config::default().with_frequency(Rate::from_khz(400)),
        Config::default().with_frequency(Rate::from_mhz(25)),
    );

    loop {
        match sd_card.init_card().await {
            Ok(mut card) => {
                info!("Detected new card");
                match (async || {
                    let capacity = card.capacity().await?;
                    info!("Card capacity: {} B", capacity);

                    let mut first_sector = [Default::default(); 512];
                    card.read(0, &mut first_sector).await?;
                    let mbr: GenericMbr = transmute!(first_sector);
                    for partition in mbr
                        .partition_entries
                        .iter()
                        .filter(|entry| !entry.is_empty())
                    {
                        println!("Partition: {:?}", partition);

                        let mut start_sector = [Default::default(); size_of::<Bpb>()];
                        let partition_start = partition.start_sector() as u64 * 512;
                        card.read(partition_start, &mut start_sector).await?;
                        let bpb: Bpb = transmute!(start_sector);
                        // println!("BPB: {:#?}", bpb);
                        println!("FAT Type: {:#?}", bpb.fat_type());
                        let mut cluster_number = bpb.root_dir_start_cluster();
                        let mut entry_index_within_cluster = 0;
                        let mut parser = DirEntryParser::default();
                        let mut block_address = None;
                        let mut block = [Default::default(); BLOCK_SIZE];
                        loop {
                            if entry_index_within_cluster == bpb.bytes_per_cluster() / 32 {
                                let mut cluster_info_buffer =
                                    [Default::default(); Bpb::MAX_CLUSTER_INFO_SIZE];
                                let cluster_info =
                                    &mut cluster_info_buffer[..bpb.cluster_info_size()];
                                card.read(bpb.cluster_info_start(cluster_number), cluster_info)
                                    .await?;
                                match bpb.next_cluster_number(cluster_info).unwrap() {
                                    Some(next_cluster_number) => {
                                        cluster_number = next_cluster_number;
                                        entry_index_within_cluster = 0;
                                    }
                                    None => break,
                                }
                            }
                            let entry_address = bpb.cluster_start(cluster_number)
                                + entry_index_within_cluster as u64 * 32;
                            let required_block_address =
                                entry_address / BLOCK_SIZE as u64 * BLOCK_SIZE as u64;
                            if block_address != Some(required_block_address) {
                                card.read(partition_start + required_block_address, &mut block)
                                    .await?;
                                block_address = Some(required_block_address);
                            };
                            let start = entry_address as usize % BLOCK_SIZE;
                            let entry = block[start..start + 32].try_into().unwrap();
                            match parser.parse_entry(entry).unwrap() {
                                ParseEntryOutput::KeepReadingToParseCurrentEntry(new_parser) => {
                                    parser = new_parser;
                                }
                                ParseEntryOutput::KeepReadingToParseNextEntry(entry) => {
                                    parser = Default::default();
                                    if let Some(entry) = entry {
                                        // Max len of UTF-8 from UTF-16 of [u16; N] is [u8; N * 3]
                                        let name =
                                            String::<{ 255 * 3 }>::from_utf16(&entry.name).unwrap();
                                        if entry.volume_id {
                                            println!("Volume Label: {:?}", name);
                                        } else {
                                            println!("File: {:?}", name);
                                        }
                                    }
                                }
                                ParseEntryOutput::DoneReadingEntries => {
                                    break;
                                }
                            }
                            entry_index_within_cluster += 1;
                        }
                        info!("Done reading all entries of root dir");
                        card.test().await?;
                    }
                    Ok::<_, spi_sd_card::Error<_, _>>(())
                })()
                .await
                {
                    Ok(()) => loop {
                        Timer::after(Duration::from_secs(1)).await;
                        match card.get_status().await {
                            Ok(status) => {
                                if status.is_empty() {
                                    info!("Card still present");
                                } else {
                                    println!("Card is present, but status error: {status:?}");
                                }
                            }
                            Err(spi_sd_card::Error::SendStatusResponseTimeout) => {
                                info!(
                                    "Timeout after asking card to send status. Assuming card was removed."
                                );
                                break;
                            }
                            Err(e) => {
                                println!("Error getting status: {e:#?}. Card may not be present");
                            }
                        }
                    },
                    Err(e) => {
                        println!("Error reading card: {e:#?}");
                    }
                }
            }
            Err(e) => {
                println!("Error getting card: {e:#?}. Maybe there is no card present");
            }
        }
        Timer::after(Duration::from_secs(1)).await;
    }
}
