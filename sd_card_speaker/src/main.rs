#![no_std]
#![no_main]
mod queue;

use core::{cmp::min, mem::transmute};

use defmt::{info, warn};
use embassy_executor::Spawner;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, mutex::Mutex, pipe::Pipe};
use embassy_time::{Delay, Duration, Instant, Timer};
use esp_backtrace as _;
use esp_hal::{
    dma::{DmaError, DmaRxBuf, DmaTxBuf},
    dma_buffers, dma_circular_buffers,
    gpio::{Level, Output, OutputConfig},
    i2s::{
        self,
        master::{DataFormat, I2s},
    },
    interrupt::software::SoftwareInterruptControl,
    spi::master::{Config, Spi},
    time::Rate,
    timer::timg::TimerGroup,
};
use esp_println as _;
use esp_println::println;
use futures::future::join;
use heapless::String;
use pure_fat::{
    Bpb, DirEntryParser, ParseEntryOutput, ReadClusterInfo, ReadFile, ReadFileInput,
    ReadFileOutput, ReadFilePart, StateMachine as _, StreamFile, StreamInput, StreamOutput,
};
use pure_mbr::GenericMbr;
use pure_wav::{
    ChunkInfo, GetMetaDataForI2s, GetMetaDataForI2sOutput, Header, ReadRequest, StateMachine as _,
    WaveFile, parse_top_header,
};
use spi_sd_card::{BLOCK_SIZE, Disk, EmbassySharedSpiBus, SpiSdCard};
use zerocopy::{FromBytes, Immutable, IntoBytes, transmute, transmute_mut, transmute_ref};

use crate::queue::Queue;

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
            .with_sck(peripherals.GPIO7)
            .with_mosi(peripherals.GPIO6)
            .with_miso(peripherals.GPIO5)
            .with_dma(peripherals.DMA_CH0)
            .with_buffers(dma_rx_buf, dma_tx_buf)
            .into_async(),
    );

    let cs = Output::new(peripherals.GPIO0, Level::High, OutputConfig::default());

    let mut sd_card = SpiSdCard::new(
        EmbassySharedSpiBus::new(&spi_bus),
        cs,
        Delay,
        Config::default().with_frequency(Rate::from_khz(400)),
        Config::default().with_frequency(Rate::from_mhz(25)),
    );

    let mut card = sd_card.init_card().await.unwrap();
    info!("Got card");
    let capacity = card.capacity().await.unwrap();
    info!("Card capacity: {} B", capacity);

    let mut first_sector = [Default::default(); 512];
    card.read(0, &mut first_sector).await.unwrap();
    let mbr: GenericMbr = transmute!(first_sector);
    let partition = mbr
        .partition_entries
        .iter()
        .find(|entry| !entry.is_empty())
        .unwrap();

    println!("Partition: {:?}", partition);

    let mut start_sector = [Default::default(); size_of::<Bpb>()];
    let partition_start = partition.start_sector() as u64 * 512;
    card.read(partition_start, &mut start_sector).await.unwrap();
    let bpb: Bpb = transmute!(start_sector);
    // println!("BPB: {:#?}", bpb);
    // println!("FAT Type: {:#?}", bpb.fat_type());
    let mut cluster_number = bpb.root_dir_start_cluster();
    let mut entry_index_within_cluster = 0;
    let mut parser = DirEntryParser::default();
    let mut block_address = None;
    let mut block = [Default::default(); BLOCK_SIZE];
    let (wave_file_start_cluster_number, wave_file_size) = loop {
        if entry_index_within_cluster == bpb.bytes_per_cluster() / 32 {
            let mut cluster_info_buffer = [Default::default(); Bpb::MAX_CLUSTER_INFO_SIZE];
            let cluster_info = &mut cluster_info_buffer[..bpb.cluster_info_size()];
            card.read(bpb.cluster_info_start(cluster_number), cluster_info)
                .await
                .unwrap();
            match bpb.next_cluster_number(cluster_info).unwrap() {
                Some(next_cluster_number) => {
                    cluster_number = next_cluster_number;
                    entry_index_within_cluster = 0;
                }
                None => break None,
            }
        }
        let entry_address =
            bpb.cluster_start(cluster_number) + entry_index_within_cluster as u64 * 32;
        let required_block_address = entry_address / BLOCK_SIZE as u64 * BLOCK_SIZE as u64;
        if !block_address.is_some_and(|block_address| block_address == required_block_address) {
            card.read(partition_start + required_block_address, &mut block)
                .await
                .unwrap();
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
                    let name = String::<{ 255 * 3 }>::from_utf16(&entry.name);
                    println!("Name: {:?}", name);
                    if char::decode_utf16(entry.name).eq("audio.wav".chars().map(Ok)) {
                        break Some((entry.first_cluster_number, entry.size));
                    }
                }
            }
            ParseEntryOutput::DoneReadingEntries => {
                break None;
            }
        }
        entry_index_within_cluster += 1;
    }
    .unwrap();

    let mut get_meta_data = GetMetaDataForI2s::new();
    let meta_data = loop {
        let output = get_meta_data.output();
        match output {
            GetMetaDataForI2sOutput::Done(output) => break output,
            GetMetaDataForI2sOutput::Read(ReadRequest { address, size }) => {
                let mut buffer = [Default::default(); GetMetaDataForI2s::MAX_READ_LEN];
                let mut read_file =
                    ReadFile::new(&bpb, wave_file_start_cluster_number, address, size);
                loop {
                    let output = read_file.output();
                    match output {
                        ReadFileOutput::Done => {
                            break;
                        }
                        ReadFileOutput::ReadFilePart(ReadFilePart {
                            address_in_buffer,
                            address_in_partition,
                            copy_len,
                        }) => {
                            card.read(
                                partition_start + address_in_partition,
                                &mut buffer[address_in_buffer..address_in_buffer + copy_len],
                            )
                            .await
                            .unwrap();
                            read_file.input(ReadFileInput::DoneReadingPart);
                        }
                        ReadFileOutput::ReadClusterInfo(ReadClusterInfo {
                            address_in_partition,
                            len,
                        }) => {
                            let mut info_buffer = [Default::default(); Bpb::MAX_CLUSTER_INFO_SIZE];
                            let info_buffer = &mut info_buffer[..len];
                            card.read(partition_start + address_in_partition, info_buffer)
                                .await
                                .unwrap();
                            read_file.input(ReadFileInput::ReadClusterInfo(info_buffer));
                        }
                    }
                }
                get_meta_data.input(&buffer[..size as usize]);
            }
        }
    }
    .unwrap();
    println!("audio.wav: {meta_data:#?}");

    const QUEUE_LEN: usize = BLOCK_SIZE * 1;
    let mut queue = Queue::<QUEUE_LEN>::new();
    let (mut writer, mut reader) = queue.split();

    join(
        async {
            let mut stream = StreamFile::new(&bpb, wave_file_size, wave_file_start_cluster_number);
            let mut total_bytes_read = 0;
            let mut total_read_time = Duration::default();
            loop {
                match stream.output().unwrap() {
                    StreamOutput::Cluster(cluster) => {
                        // println!("File cluster: {cluster:X?}");

                        let mut bytes_read = 0;
                        while bytes_read < cluster.len {
                            let before = Instant::now();
                            let mut buffer = [i16::default(); BLOCK_SIZE / 2];
                            let bytes_to_read =
                                min(cluster.len - bytes_read, buffer.as_bytes().len() as u32);
                            card.read(
                                partition_start + cluster.address + bytes_read as u64,
                                &mut buffer.as_mut_bytes()[..bytes_to_read as usize],
                            )
                            .await
                            .unwrap();

                            for byte in &mut buffer[..(bytes_to_read / 2) as usize] {
                                *byte /= 4;
                            }

                            total_bytes_read += buffer.as_bytes().len();
                            let time = before.elapsed();
                            total_read_time += time;
                            // println!(
                            //     "read and processed {} B / {} us (avg: {} B / {} us)",
                            //     buffer.as_bytes().len(),
                            //     before.elapsed().as_micros(),
                            //     total_bytes_read,
                            //     total_read_time.as_micros()
                            // );

                            let mut bytes_written = 0;
                            // println!("Pushing 0x{bytes_to_read:X} bytes to the pipe");
                            while bytes_written < bytes_to_read {
                                // println!("bytes written: {bytes_written}");
                                writer.wait_until_available(1).await;
                                let write_buffer = writer.write_buffer();
                                let copy_len_0 = min(
                                    write_buffer.0.len(),
                                    (bytes_to_read - bytes_written) as usize,
                                );
                                write_buffer.0[..copy_len_0].copy_from_slice(
                                    &buffer.as_bytes()[bytes_written as usize
                                        ..bytes_written as usize + copy_len_0],
                                );
                                bytes_written += copy_len_0 as u32;
                                let copy_len_1 = min(
                                    write_buffer.1.len(),
                                    (bytes_to_read - bytes_written) as usize,
                                );
                                write_buffer.1[..copy_len_1].copy_from_slice(
                                    &buffer.as_bytes()[bytes_written as usize
                                        ..bytes_written as usize + copy_len_1],
                                );
                                bytes_written += copy_len_1 as u32;
                                let total_copy_len = copy_len_0 + copy_len_1;
                                let available = writer.bytes_available();
                                writer.mark_written(total_copy_len);
                                // println!("put {total_copy_len} B in queue ({available} B space available in queue)");
                            }
                            // println!("Done pushing to pipe");

                            bytes_read += bytes_to_read;
                        }

                        stream.input(StreamInput::Next);
                    }
                    StreamOutput::ReadClusterInfo(ReadClusterInfo {
                        address_in_partition,
                        len,
                    }) => {
                        let mut info_buffer = [Default::default(); Bpb::MAX_CLUSTER_INFO_SIZE];
                        let info_buffer = &mut info_buffer[..len];
                        card.read(partition_start + address_in_partition, info_buffer)
                            .await
                            .unwrap();
                        stream.input(StreamInput::ClusterInfo(info_buffer));
                    }
                    StreamOutput::Done => {
                        break;
                    }
                }
            }
        },
        async {
            let i2s = I2s::new(
                peripherals.I2S0,
                peripherals.DMA_CH1,
                i2s::master::Config::new_tdm_msb()
                    .with_sample_rate(Rate::from_hz(meta_data.fmt_data.n_samples_per_sec.get()))
                    .with_data_format(match meta_data.fmt_data.w_bits_per_sample.get() {
                        16 => DataFormat::Data16Channel16,
                        _ => todo!(),
                    }),
            )
            .unwrap()
            .into_async();
            let (_rx_buffer, _rx_descriptors, tx_buffer, tx_descriptors) =
                dma_circular_buffers!(0, 16 * 1024);
            let tx_buffer = tx_buffer;
            let tx = i2s
                .i2s_tx
                .with_bclk(peripherals.GPIO2)
                .with_dout(peripherals.GPIO1)
                .with_ws(peripherals.GPIO3)
                .build(tx_descriptors);
            // Wait until the buffer is completely full
            reader.wait_until_available(QUEUE_LEN).await;
            let mut transfer = tx.write_dma_circular_async(tx_buffer).unwrap();
            loop {
                reader.wait_until_available(1).await;
                transfer
                    .available()
                    .await
                    .inspect_err(|e| match e {
                        i2s::master::Error::DmaError(DmaError::Late) => {
                            warn!("late");
                        }
                        _ => {}
                    })
                    .unwrap();
                transfer
                    .push_with(|buffer| {
                        let read_buffer = reader.read_buffer();
                        let copy_len_0 = min(buffer.len(), read_buffer.0.len());
                        buffer[..copy_len_0].copy_from_slice(&read_buffer.0[..copy_len_0]);
                        let copy_len_1 = min(buffer.len() - copy_len_0, read_buffer.1.len());
                        buffer[copy_len_0..copy_len_0 + copy_len_1]
                            .copy_from_slice(&read_buffer.1[..copy_len_1]);
                        let total_copy_len = copy_len_0 + copy_len_1;
                        reader.mark_read(total_copy_len);
                        // if total_copy_len > 0 {
                        // println!(
                        //     "Pushing {total_copy_len} B / {} B available to I2S DMA",
                        //     buffer.len()
                        // );
                        // }
                        if total_copy_len == 0 {
                            // warn!("I2S DMA buffer had bytes available but we had no bytes to push to it");
                        }
                        total_copy_len
                    })
                    .await
                    .unwrap();
            }
        },
    )
    .await;
}
