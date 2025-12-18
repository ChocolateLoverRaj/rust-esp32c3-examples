#![no_std]
#![no_main]

use defmt::{Format, info};
use embassy_executor::Spawner;
use esp_backtrace as _;
use esp_hal::{
    dma_buffers, dma_tx_buffer,
    i2s::master::{Channels, Config, DataFormat, I2s, UnitConfig},
    interrupt::software::SoftwareInterruptControl,
    time::Rate,
    timer::timg::TimerGroup,
};
use esp_println as _;
use zerocopy::{
    FromBytes, Immutable, KnownLayout, Unaligned, transmute, transmute_ref, try_transmute_ref,
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

    info!("Hello from a Rust no_std environment with esp_rtos (basically embassy for ESP32).");

    let wav_file = include_bytes!("../test.wav");

    let mut i2s = I2s::new(peripherals.I2S0, peripherals.DMA_CH0, Config::new_tdm_msb())
        .unwrap()
        .into_async();
    let (rx_buffer, rx_descriptors, tx_buffer, tx_descriptors) = dma_buffers!(0, 100 * 1024);
    let mut tx = Some(
        i2s.i2s_tx
            .with_bclk(peripherals.GPIO20)
            .with_dout(peripherals.GPIO1)
            .with_ws(peripherals.GPIO7)
            .build(tx_descriptors),
    );
    let mut tx_buffer = Some(tx_buffer);

    #[derive(Debug, Clone, Copy, FromBytes, Format, Immutable, KnownLayout)]
    #[repr(C)]
    struct Riff {
        pub chunk_id: [u8; 4],
        pub chunk_size: [u8; 4],
        pub wave_id: [u8; 4],
    }

    let (riff, root_chunk_data) = Riff::ref_from_prefix(wav_file).unwrap();
    info!("Riff: {:#?}", riff);
    assert_eq!(riff.chunk_id.as_slice(), b"RIFF");
    let chunk_size = u32::from_le_bytes(riff.chunk_size);
    assert_eq!(riff.wave_id.as_slice(), b"WAVE");

    info!("Chunk size: {}", chunk_size);

    #[derive(Debug, Clone, Copy, FromBytes, Format, Immutable, KnownLayout)]
    #[repr(C)]
    struct ChunkHeader {
        pub chunk_id: [u8; 4],
        pub chunk_size: [u8; 4],
    }

    let mut offset = 0;
    loop {
        if offset == root_chunk_data.len() {
            break;
        }
        let chunk_header = ChunkHeader::ref_from_bytes(
            &root_chunk_data[offset..offset + size_of::<ChunkHeader>()],
        )
        .unwrap();
        let chunk_id = str::from_utf8(&chunk_header.chunk_id).unwrap();
        let chunk_size = u32::from_le_bytes(chunk_header.chunk_size);
        match chunk_id {
            "fmt " => {
                #[derive(Debug, Clone, Copy, FromBytes, Format, Immutable, KnownLayout)]
                #[repr(C)]
                struct FmtData {
                    pub format_tag: [u8; 2],
                    pub n_channels: [u8; 2],
                    pub n_samples_per_sec: [u8; 4],
                    pub n_avg_bytes_per_sec: [u8; 4],
                    pub n_block_align: [u8; 2],
                    pub w_bits_per_sample: [u8; 2],
                    pub cb_size: [u8; 2],
                    pub w_valid_bits_per_sample: [u8; 2],
                    pub dw_channel_mask: [u8; 4],
                    pub sub_format: [u8; 16],
                }
                let data_pos = offset + size_of::<ChunkHeader>();
                let data = FmtData::ref_from_bytes(
                    &root_chunk_data[data_pos..data_pos + size_of::<FmtData>()],
                )
                .unwrap();
                info!("{:#?}", data);
                let format_tag = u16::from_le_bytes(data.format_tag);
                if format_tag != 0x1 {
                    panic!("Unsupported format");
                }
                let samples_per_sec = u32::from_le_bytes(data.n_samples_per_sec);
                info!("samples per sec: {}", samples_per_sec);
                let bits_per_sample = u16::from_le_bytes(data.w_bits_per_sample);
                info!("bits per samples: {}", bits_per_sample);
                tx.as_mut()
                    .unwrap()
                    .apply_config(
                        &UnitConfig::new_tdm_philips()
                            .with_sample_rate(Rate::from_hz(samples_per_sec))
                            .with_data_format(match bits_per_sample {
                                16 => DataFormat::Data16Channel16,
                                _ => todo!(),
                            }),
                    )
                    .unwrap();
            }
            "data" => {
                let data_pos = offset + size_of::<ChunkHeader>();
                let audio_len = (chunk_size / 2 * 2) as usize;
                info!("playing {} bytes", audio_len);
                let mut data_remaining = &root_chunk_data[data_pos..data_pos + audio_len];
                let mut transfer = tx
                    .take()
                    .unwrap()
                    .write_dma_circular_async(tx_buffer.take().unwrap())
                    .unwrap();
                while !data_remaining.is_empty() {
                    let available = transfer.available().await.unwrap();
                    let bytes_to_push = data_remaining.len().min(available);
                    info!("available: {}. pushing: {}.", available, bytes_to_push);
                    let mut bytes_left_to_push = bytes_to_push;
                    while bytes_left_to_push >= size_of::<i16>() {
                        let mut value = i16::from_le_bytes(
                            *<&[u8; 2]>::try_from(&data_remaining[..size_of::<i16>()]).unwrap(),
                        );
                        value /= 16;
                        transfer.push(&value.to_le_bytes()).await.unwrap();
                        data_remaining = &data_remaining[size_of::<i16>()..];
                        bytes_left_to_push -= size_of::<i16>();
                    }
                }
                info!("done playing");
            }
            _ => info!("unknown chunk id: {}", chunk_id),
        }
        offset += size_of::<ChunkHeader>() + chunk_size as usize;
    }

    // tx.apply_config(UnitConfig::new_tdm_msb().with_channels(Channels::STEREO).with_sample_rate(R))
}
