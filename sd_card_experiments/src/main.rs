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
    Acmd41Output, Command0, Command0Process, Command8, Command8Process, Command59,
    Command59Process, Ocr, R1, R3, SimpleCmdProcess, SimpleCommand, format_acmd_41, format_cmd_8,
    format_cmd_55, format_cmd_58, format_cmd_59, format_command_0, prepare_command_0,
    process_acmd_41_res, process_cmd_0_response, process_cmd_8_res, process_cmd_55_response,
    process_cmd_59_res,
};
// use spi_sd_card::{
//     Action, ActionResponse, ResetAndInit, format_command, prepare_command_0, process_command_0,
// };

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
    .with_dma(peripherals.DMA_CH2)
    .with_buffers(dma_rx_buf, dma_tx_buf)
    .into_async();

    let mut cs = Output::new(peripherals.GPIO1, Level::High, OutputConfig::default());

    // Send 74 clock cycles
    // Rounded up to 10 bytes
    spi_bus.write_async(&[0xFF; 74 / 8]).await.unwrap();

    cs.set_low();

    {
        spi_bus.write_async(&format_command_0()).await.unwrap();
        let mut c = Command0;
        let r = loop {
            let mut buffer = [0xFF; 1];
            spi_bus.transfer_in_place_async(&mut buffer).await.unwrap();
            match c.process_bytes(&buffer) {
                Command0Process::InProgress(new_c) => {
                    c = new_c;
                }
                Command0Process::Done(r1) => break r1,
            }
        };
        info!("cmd0 r: {:X}", Debug2Format(&r));
        process_cmd_0_response(r).unwrap();
    }

    {
        spi_bus.write_async(&format_cmd_59(true)).await.unwrap();
        let mut c = Command59;
        let r = loop {
            let mut buffer = [0xFF; 1];
            spi_bus.transfer_in_place_async(&mut buffer).await.unwrap();
            match c.process_bytes(&buffer) {
                Command59Process::InProgress(new_c) => {
                    c = new_c;
                }
                Command59Process::Done(r1) => break r1,
            }
        };
        info!("cmd59 r: {:X}", Debug2Format(&r));
        process_cmd_59_res(r).unwrap();
    }

    {
        spi_bus.write_async(&format_cmd_8()).await.unwrap();
        let mut c = Command8::default();
        let r = loop {
            let mut buffer = [0xFF; 1];
            spi_bus.transfer_in_place_async(&mut buffer).await.unwrap();
            match c.process_bytes(&buffer) {
                Command8Process::InProgress(new_c) => {
                    c = new_c;
                }
                Command8Process::Done(r1) => break r1,
            }
        };
        info!("cmd8 r: {:X}", Debug2Format(&r));
        process_cmd_8_res(r).unwrap();
    }

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
    }

    loop {
        {
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
            info!("cmd55 r: {:X}", Debug2Format(&r));
            process_cmd_55_response(r).unwrap();
        }
        {
            spi_bus.write_async(&format_acmd_41()).await.unwrap();
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
            info!("ACMD41 r: {:X}", Debug2Format(&r));
            let o = process_acmd_41_res(r).unwrap();
            info!("ACMD41 o: {}", o);
            if matches!(o, Acmd41Output::Initialized) {
                break;
            }
        }
    }

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
        let is_hcs = r.ocr.supports_sdhc_or_sdxc().unwrap();
        info!("cmd58 is HCS?: {}", is_hcs);
    }
}
