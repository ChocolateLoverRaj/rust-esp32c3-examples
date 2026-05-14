#![no_std]
#![no_main]

use collect_array_ext_trait::CollectArray;
use defmt::{Debug2Format, info, warn};
use embassy_executor::Spawner;
use embassy_time::{Duration, Instant, TICK_HZ, Timer};
use esp_backtrace as _;
use esp_hal::{
    clock::CpuClock,
    dma::{DmaRxBuf, DmaTxBuf},
    dma_buffers,
    gpio::{Level, Output, OutputConfig},
    interrupt::software::SoftwareInterruptControl,
    spi::master::{Config, Spi},
    time::Rate,
    timer::timg::TimerGroup,
};
use esp_println as _;
use spi_sd_card::{
    Acmd41Output, Cmd8Res, Csd, CsdCommon, KeepAction, Ocr, R1, R3, ReadMultiCmd, ReadMultiOutput,
    ReadSingleCmd, ReadSingleProcess, SimpleCmdProcess, SimpleCommand, format_acmd_41,
    format_cmd_8, format_cmd_9, format_cmd_12, format_cmd_17, format_cmd_18, format_cmd_55,
    format_cmd_58, format_cmd_59, format_command_0, process_acmd_41_res, process_cmd_0_response,
    process_cmd_8_res, process_cmd_55_response, process_cmd_59_res,
};
use split_slice::SplitSlice;

// use spi_sd_card::{
//     Action, ActionResponse, ResetAndInit, format_command, prepare_command_0, process_command_0,
// };

esp_bootloader_esp_idf::esp_app_desc!();

#[esp_rtos::main]
async fn main(spawner: Spawner) {
    let _ = spawner;

    let peripherals = esp_hal::init(esp_hal::Config::default().with_cpu_clock(CpuClock::max()));

    // Needed for esp_rtos
    let timg0 = TimerGroup::new(peripherals.TIMG0);
    let software_interrupt = SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    esp_rtos::start(timg0.timer0, software_interrupt.software_interrupt0);

    let (rx_buffer, rx_descriptors, tx_buffer, tx_descriptors) = dma_buffers!(10 * 1024);
    let dma_rx_buf = DmaRxBuf::new(rx_descriptors, rx_buffer).unwrap();
    let dma_tx_buf = DmaTxBuf::new(tx_descriptors, tx_buffer).unwrap();

    let mut spi_bus = Spi::new(
        peripherals.SPI2,
        Config::default().with_frequency(Rate::from_khz(400)),
    )
    .unwrap()
    .with_sck(peripherals.GPIO6)
    .with_mosi(peripherals.GPIO7)
    .with_miso(peripherals.GPIO2)
    .with_dma(peripherals.DMA_CH2)
    .with_buffers(dma_rx_buf, dma_tx_buf)
    .into_async();

    let mut cs = Output::new(peripherals.GPIO1, Level::High, OutputConfig::default());

    // Send 74 clock cycles
    // Rounded up to 10 bytes
    spi_bus.write_async(&[0xFF; 74 / 8]).await.unwrap();

    'a: loop {
        loop {
            cs.set_low();
            spi_bus.write_async(&format_command_0()).await.unwrap();
            let mut c = SimpleCommand::<{ size_of::<R1>() }>::default();
            let r = R1::from_bits_retain(loop {
                let mut buffer = [0xFF; 1];
                spi_bus.transfer_in_place_async(&mut buffer).await.unwrap();
                match c.process_bytes(&buffer) {
                    SimpleCmdProcess::InProgress(new_c) => {
                        info!("cmd0 in progress");
                        c = new_c;
                    }
                    SimpleCmdProcess::Done([r1]) => break r1,
                }
            });
            info!("cmd0 r: {:X}", Debug2Format(&r));
            cs.set_high();
            spi_bus.write_async(&[0xFF; 1]).await.unwrap();
            if process_cmd_0_response(r).is_ok() {
                break;
            }
        }

        cs.set_low();
        {
            spi_bus.write_async(&format_cmd_59(true)).await.unwrap();
            let mut c = SimpleCommand::<{ size_of::<R1>() }>::default();
            let r = R1::from_bits_retain(loop {
                let mut buffer = [0xFF; 20];
                spi_bus.transfer_in_place_async(&mut buffer).await.unwrap();
                match c.process_bytes(&buffer) {
                    SimpleCmdProcess::InProgress(new_c) => {
                        c = new_c;
                    }
                    SimpleCmdProcess::Done([r1]) => break r1,
                }
            });
            info!("cmd59 r: {:X}", Debug2Format(&r));
            process_cmd_59_res(r).unwrap();
        }
        cs.set_high();
        spi_bus.write_async(&[0xFF; 1]).await.unwrap();

        cs.set_low();
        {
            spi_bus.write_async(&format_cmd_8()).await.unwrap();
            let mut c = SimpleCommand::<{ size_of::<Cmd8Res>() }>::default();
            let r = loop {
                let mut buffer = [0xFF; 10];
                spi_bus.transfer_in_place_async(&mut buffer).await.unwrap();
                match c.process_bytes(&buffer) {
                    SimpleCmdProcess::InProgress(new_c) => {
                        c = new_c;
                    }
                    SimpleCmdProcess::Done(r1) => break r1,
                }
            };
            info!("cmd8 r: {:X}", Debug2Format(&r));
            process_cmd_8_res(r).unwrap();
        }
        cs.set_high();
        spi_bus.write_async(&[0xFF; 1]).await.unwrap();

        cs.set_low();
        {
            spi_bus.write_async(&format_cmd_58()).await.unwrap();
            let mut c = SimpleCommand::<{ size_of::<R3>() }>::default();
            let r = loop {
                let mut buffer = [0xFF; 1];
                spi_bus.transfer_in_place_async(&mut buffer).await.unwrap();
                match c.process_bytes(&buffer) {
                    SimpleCmdProcess::InProgress(new_c) => {
                        c = new_c;
                    }
                    SimpleCmdProcess::Done(bytes) => {
                        let r1 = bytes[0];
                        let ocr = u32::from_be_bytes(bytes[1..5].try_into().unwrap());
                        break R3 {
                            r1: R1::from_bits_retain(r1),
                            ocr: Ocr::from_bits_retain(ocr),
                        };
                    }
                }
            };
            info!("cmd58 r: {:X}", Debug2Format(&r));
            assert!(r.ocr.supports_3_3v());
        }
        cs.set_high();
        spi_bus.write_async(&[0xFF; 1]).await.unwrap();

        loop {
            loop {
                cs.set_low();
                spi_bus.write_async(&format_cmd_55()).await.unwrap();
                let mut c = SimpleCommand::<{ size_of::<R1>() }>::default();
                let r = loop {
                    let mut buffer = [0xFF; 1];
                    spi_bus.transfer_in_place_async(&mut buffer).await.unwrap();
                    match c.process_bytes(&buffer) {
                        SimpleCmdProcess::InProgress(new_c) => {
                            c = new_c;
                        }
                        SimpleCmdProcess::Done([r1]) => {
                            break R1::from_bits_retain(r1);
                        }
                    }
                };
                cs.set_high();
                spi_bus.write_async(&[0xFF; 1]).await.unwrap();
                info!("cmd55 r: {:X}", Debug2Format(&r));
                if process_cmd_55_response(r).is_ok() {
                    break;
                }
                Timer::after_millis(10).await;
            }
            {
                cs.set_low();
                spi_bus.write_async(&format_acmd_41()).await.unwrap();
                let mut c = SimpleCommand::<{ size_of::<R1>() }>::default();
                let r = loop {
                    let mut buffer = [0xFF; 10];
                    spi_bus.transfer_in_place_async(&mut buffer).await.unwrap();
                    match c.process_bytes(&buffer) {
                        SimpleCmdProcess::InProgress(new_c) => {
                            c = new_c;
                        }
                        SimpleCmdProcess::Done([r1]) => {
                            break R1::from_bits_retain(r1);
                        }
                    }
                };
                cs.set_high();
                spi_bus.write_async(&[0xFF; 1]).await.unwrap();
                info!("ACMD41 r: {:X}", Debug2Format(&r));
                if let Ok(o) = process_acmd_41_res(r) {
                    info!("ACMD41 o: {}", o);
                    if matches!(o, Acmd41Output::Initialized) {
                        break;
                    }
                } else {
                    Timer::after_millis(10).await;
                }
            }
        }

        spi_bus
            .apply_config(&Config::default().with_frequency(Rate::from_mhz(25)))
            .unwrap();

        {
            cs.set_low();
            spi_bus.write_async(&format_cmd_58()).await.unwrap();
            let mut c = SimpleCommand::<{ size_of::<R3>() }>::default();
            let r = loop {
                let mut buffer = [0xFF; 1];
                spi_bus.transfer_in_place_async(&mut buffer).await.unwrap();
                match c.process_bytes(&buffer) {
                    SimpleCmdProcess::InProgress(new_c) => {
                        c = new_c;
                    }
                    SimpleCmdProcess::Done(bytes) => {
                        let r1 = bytes[0];
                        let ocr = u32::from_be_bytes(bytes[1..5].try_into().unwrap());
                        break R3 {
                            r1: R1::from_bits_retain(r1),
                            ocr: Ocr::from_bits_retain(ocr),
                        };
                    }
                }
            };
            let is_hcs = r.ocr.supports_sdhc_or_sdxc().unwrap();
            info!("cmd58 is HCS?: {}", is_hcs);
            cs.set_high();
            spi_bus.write_async(&[0xFF; 1]).await.unwrap();
        }

        {
            let mut prev_capacity = None;
            loop {
                cs.set_low();
                spi_bus.write_async(&format_cmd_9()).await.unwrap();
                let mut c = ReadSingleCmd::new(size_of::<u128>());
                let mut data_buffer =
                    heapless::Vec::<_, { size_of::<u128>() + size_of::<u16>() }>::new();
                let mut keep_started = false;
                let mut transfer_buffer = [Default::default(); 1];
                let result = loop {
                    let transfer_len = if keep_started {
                        ((size_of::<u128>() + size_of::<u16>()) - data_buffer.len())
                            .min(transfer_buffer.len())
                    } else {
                        transfer_buffer.len()
                    };
                    // info!(
                    //     "transfer len: {} {} {}",
                    //     transfer_len,
                    //     keep_started,
                    //     data_buffer.len()
                    // );
                    assert_ne!(transfer_len, 0);
                    let mut transfer_buffer = &mut transfer_buffer[..transfer_len];
                    transfer_buffer.fill(0xFF);
                    spi_bus
                        .transfer_in_place_async(&mut transfer_buffer)
                        .await
                        .unwrap();
                    match c.process_bytes(transfer_buffer) {
                        Ok(ReadSingleProcess::InProgress { cmd, keep_start }) => {
                            c = cmd;
                            if keep_started {
                                data_buffer.extend_from_slice(transfer_buffer).unwrap();
                            } else if let Some(keep_start) = keep_start {
                                keep_started = true;
                                data_buffer
                                    .extend_from_slice(&transfer_buffer[keep_start..])
                                    .unwrap();
                            }
                        }
                        Ok(ReadSingleProcess::Done { bytes_processed }) => {
                            break Ok(if keep_started {
                                data_buffer
                                    .extend_from_slice(&transfer_buffer[..bytes_processed])
                                    .unwrap();
                                &data_buffer
                            } else {
                                &transfer_buffer[bytes_processed
                                    - (size_of::<u128>() + size_of::<u16>())
                                    ..bytes_processed]
                            });
                        }
                        Err(e) => {
                            break Err(e);
                        }
                    }
                };
                cs.set_high();
                spi_bus.write_async(&[0xFF; 1]).await.unwrap();

                match result {
                    Ok(csd_and_crc) => {
                        let csd = &csd_and_crc[..size_of::<u128>()];
                        let received_crc = u16::from_be_bytes(
                            csd_and_crc[size_of::<u128>()..].try_into().unwrap(),
                        );
                        let mut digest = spi_sd_card::CRC.digest();
                        digest.update(csd);
                        let computed_crc = digest.finalize();
                        if received_crc == computed_crc {
                            let csd = CsdCommon::new_with_raw_value(u128::from_be_bytes(
                                csd.try_into().unwrap(),
                            ));
                            let capacity = csd.card_capacity_bytes();
                            info!("Capacity: {} B", capacity);
                            if let Some(prev_capacity) = prev_capacity {
                                if prev_capacity == capacity {
                                    break 'a;
                                } else {
                                    warn!("Inconsistent reported capacity. Retrying...");
                                    break;
                                }
                            } else {
                                prev_capacity = Some(capacity);
                            }
                        } else {
                            warn!("CMD9 CRC mismatch. Retrying...");
                            break;
                        }
                    }
                    Err(e) => {
                        warn!("CMD9 error: {}. Retrying...", Debug2Format(&e));
                        break;
                    }
                }
            }
        }
    }
    // {
    //     cs.set_low();
    //     let start_time = Instant::now();
    //     spi_bus.write_async(&format_cmd_17(0)).await.unwrap();
    //     let mut c = ReadSingleCmd::new(512);
    //     let mut data_buffer = heapless::Vec::<_, { 512 + 2 }>::new();
    //     let data_start = loop {
    //         let mut transfer_buffer = [0xFF; 1];
    //         let transfer_len = data_buffer
    //             .spare_capacity_mut()
    //             .len()
    //             .min(transfer_buffer.len());
    //         let mut transfer_buffer = &mut transfer_buffer[..transfer_len];
    //         spi_bus
    //             .transfer_in_place_async(&mut transfer_buffer)
    //             .await
    //             .unwrap();
    //         let new_start = data_buffer.len();
    //         data_buffer.extend_from_slice(&transfer_buffer).unwrap();
    //         match c.process_bytes(&data_buffer, new_start) {
    //             Ok(ReadSingleProcess::InProgress { cmd, keep_start }) => {
    //                 c = cmd;
    //                 data_buffer.copy_within(keep_start.., 0);
    //                 data_buffer.truncate(data_buffer.len() - keep_start);
    //             }
    //             Ok(ReadSingleProcess::Done { data_start }) => {
    //                 break Ok(data_start);
    //             }
    //             Err(e) => {
    //                 break Err(e);
    //             }
    //         }
    //     }
    //     .unwrap();
    //     let data = &data_buffer[data_start..data_start + 512];
    //     info!(
    //         "First block: {:X} {} us",
    //         data,
    //         start_time.elapsed().as_micros(),
    //     );
    //     cs.set_high();
    //     spi_bus.write_async(&[0xFF; 1]).await.unwrap();
    // }

    {
        cs.set_low();
        let start_time = Instant::now();
        let start_block_number = 2048;
        spi_bus
            .write_async(&format_cmd_18(start_block_number))
            .await
            .unwrap();
        let mut c = ReadMultiCmd::new(512);
        // let mut data_buffer = Buffer::<_, { 512 + 2 + 10 * 1024 }>::new();
        let mut buffer = [Default::default(); 512 + 2 + 10 * 1024];
        let mut keep_start = 0;
        let mut keep_len = 0;

        let mut block_number = start_block_number;
        let mut time = Duration::default();
        let mut bytes_waited = 0;
        let mut bytes_transferred = 0;
        'cmd_18: loop {
            let (transfer_buffer_pos, transfer_buffer_end) = if keep_start + keep_len < buffer.len()
            {
                (keep_start + keep_len, buffer.len())
            } else {
                (keep_start + keep_len - buffer.len(), keep_start)
            };
            let mut transfer_buffer = &mut buffer[transfer_buffer_pos..transfer_buffer_end];
            // info!("transfer buffer len: {}", transfer_buffer.len());
            assert_ne!(transfer_buffer.len(), 0, "{keep_start} {keep_len}");
            transfer_buffer.fill(0xFF);
            spi_bus
                .transfer_in_place_async(&mut transfer_buffer)
                .await
                .unwrap();
            bytes_transferred += transfer_buffer.len();
            let transfer_buffer = &buffer[transfer_buffer_pos..transfer_buffer_end];
            let mut bytes_processed = 0;
            while bytes_processed < transfer_buffer.len() {
                // info!(
                //     "calling process_bytes. bytes_processed: {}",
                //     bytes_processed
                // );
                let t = Instant::now();
                let result = c.process_bytes(&transfer_buffer[bytes_processed..]);
                time += t.elapsed();
                match result {
                    Ok(ReadMultiOutput {
                        cmd,
                        keep_action,
                        bytes_processed: bytes_processed_just_now,
                        bytes_waited: bytes_waited_just_now,
                    }) => {
                        // info!("{} {}", keep_action, bytes_processed_just_now);
                        c = cmd;
                        bytes_processed += bytes_processed_just_now;
                        bytes_waited += bytes_waited_just_now;

                        match keep_action {
                            None => {
                                if keep_len > 0 {
                                    keep_len += bytes_processed_just_now
                                }
                            }
                            Some(KeepAction::StartKeeping { position }) => {
                                keep_start = transfer_buffer_pos + bytes_processed
                                    - bytes_processed_just_now
                                    + position;
                                keep_len = bytes_processed_just_now - position;
                            }
                            Some(KeepAction::Take) => {
                                if keep_len == 0 {
                                    keep_len = 512 + size_of::<u16>();
                                    keep_start = transfer_buffer_pos + bytes_processed - keep_len;
                                } else {
                                    keep_len += bytes_processed_just_now;
                                }
                                let block_and_crc = if keep_start + keep_len <= buffer.len() {
                                    SplitSlice(&buffer[keep_start..keep_start + keep_len], &[])
                                } else {
                                    SplitSlice(
                                        &buffer[keep_start..],
                                        &buffer[..keep_start + keep_len - buffer.len()],
                                    )
                                };
                                // println!(
                                //     "block_and_crc: {} + {} = {}",
                                //     block_and_crc.0.len(),
                                //     block_and_crc.1.len(),
                                //     block_and_crc.len()
                                // );
                                let (block, crc) = block_and_crc.split_at(512);

                                let mut digest = spi_sd_card::CRC.digest();
                                digest.update(block.0);
                                digest.update(block.1);
                                let computed_crc = digest.finalize();
                                let received_crc = u16::from_be_bytes(
                                    crc.into_iter().copied().collect_array().unwrap(),
                                );

                                assert_eq!(computed_crc, received_crc);
                                // info!(
                                //     "read block {} {:X} {:X} {:X} {:X}",
                                //     block_number,
                                //     computed_crc,
                                //     received_crc,
                                //     block_and_crc.0,
                                //     block_and_crc.1
                                // );

                                block_number += 1;

                                keep_len = 0;

                                if block_number == start_block_number + 100 {
                                    break 'cmd_18 Ok(());
                                }
                            }
                        }
                    }
                    Err(e) => {
                        break 'cmd_18 Err(e);
                    }
                }
            }
        }
        .unwrap();
        info!(
            "Read multiple in {} us, process_bytes: {} us. Bytes waited: {}. Bytes transferred: {}",
            start_time.elapsed().as_micros(),
            time.as_micros(),
            bytes_waited,
            bytes_transferred
        );
    }
}
