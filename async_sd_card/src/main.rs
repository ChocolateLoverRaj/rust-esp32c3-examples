#![no_std]
#![no_main]

use defmt::{error, info, warn};
use embassy_executor::Spawner;
use embassy_time::{Duration, Instant, Timer};
use embedded_hal_async::spi::SpiBus;
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
use spi_sd_card::{
    Cid, CsdV2, command_0, command_8, command_9, command_13, command_17, command_18, command_55,
    command_58, command_59, command_a41,
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

    let (rx_buffer, rx_descriptors, tx_buffer, tx_descriptors) = dma_buffers!(512);
    let dma_rx_buf = DmaRxBuf::new(rx_descriptors, rx_buffer).unwrap();
    let dma_tx_buf = DmaTxBuf::new(tx_descriptors, tx_buffer).unwrap();

    let mut spi_bus = Spi::new(
        peripherals.SPI2,
        Config::default().with_frequency(Rate::from_khz(400)),
    )
    .unwrap()
    .with_sck(peripherals.GPIO7)
    .with_mosi(peripherals.GPIO6)
    .with_miso(peripherals.GPIO5)
    .with_dma(peripherals.DMA_CH0)
    .with_buffers(dma_rx_buf, dma_tx_buf)
    .into_async();

    let mut cs = Output::new(peripherals.GPIO0, Level::High, OutputConfig::default());

    // The spec says to wait 1ms from when the SD card gets power
    // Realistically it has already been 1ms but just to be sure we can wait 1ms
    info!("waiting for 1ms");
    Timer::after(Duration::from_millis(1)).await;

    info!("waiting at least 74 clock cycles");
    // Send 0xFF for at least 74 clock cycles according to the spec
    // So 9 bytes
    SpiBus::write(&mut spi_bus, &[0xFF; 9]).await.unwrap();

    info!("sending CMD0");
    loop {
        match command_0(&mut spi_bus, &mut cs).await {
            Ok(()) => {
                break;
            }
            Err(spi_sd_card::Error::BadR1(r1)) => {
                warn!("Got response: 0b{:08b}. Retrying...", r1.bits());
            }
            result => result.unwrap(),
        }
    }

    // Simulate talking to a different SPI device
    SpiBus::write(&mut spi_bus, &[0xFF; 1000]).await.unwrap();

    info!("Enabling CRC");
    command_59(&mut spi_bus, &mut cs, true).await.unwrap();

    // Simulate talking to a different SPI device
    SpiBus::write(&mut spi_bus, &[0xFF; 1000]).await.unwrap();

    info!("sending CMD8");
    // The check pattern can be anything we want
    let check_pattern = 0xE2;
    match command_8(&mut spi_bus, &mut cs, check_pattern).await {
        Ok(()) => {
            info!("CMD8 Ok");

            // Simulate talking to a different SPI device
            SpiBus::write(&mut spi_bus, &[0xFF; 1000]).await.unwrap();

            info!("sending CMD58");
            let ocr = command_58(&mut spi_bus, &mut cs).await.unwrap();
            info!("OCR: 0b{:032b}", ocr.bits());
            assert!(ocr.supports_3_3v());

            // Simulate talking to a different SPI device
            SpiBus::write(&mut spi_bus, &[0xFF; 1000]).await.unwrap();

            loop {
                info!("sending CMD55");
                command_55(&mut spi_bus, &mut cs).await.unwrap();

                // Simulate talking to a different SPI device
                SpiBus::write(&mut spi_bus, &[0xFF; 1000]).await.unwrap();

                info!("sending ACMD41");
                let is_idle = command_a41(&mut spi_bus, &mut cs, true).await.unwrap();
                if is_idle {
                    info!("SD card is not ready yet");

                    // Simulate talking to a different SPI device
                    SpiBus::write(&mut spi_bus, &[0xFF; 1000]).await.unwrap();
                } else {
                    break;
                }
            }

            // Simulate talking to a different SPI device
            SpiBus::write(&mut spi_bus, &[0xFF; 1000]).await.unwrap();

            info!("sending CMD58");
            let ocr = command_58(&mut spi_bus, &mut cs).await.unwrap();
            if ocr.supports_sdhc_or_sdxc().expect("card not powered up") {
                info!("Card is in SDHC or SDXC mode");
            } else {
                info!("Card is standard capacity")
            }

            // At this point we can start using 25 MHz
            spi_bus
                .apply_config(&Config::default().with_frequency(Rate::from_mhz(25)))
                .unwrap();

            // Simulate talking to a different SPI device
            SpiBus::write(&mut spi_bus, &[0xFF; 1000]).await.unwrap();

            info!("CMD9");
            let csd = CsdV2(command_9(&mut spi_bus, &mut cs).await.unwrap());
            let capacity = csd.card_capacity_bytes();
            info!("Capacity: {}", capacity);

            info!("CMD10");
            let cid = Cid(command_9(&mut spi_bus, &mut cs).await.unwrap());
            info!("Manufacturer ID: 0x{:02X}", cid.get_mid());
            info!("OEM/Application ID: 0x{:04X}", cid.get_mid());
            let product_name = cid.get_pnm();
            match str::from_utf8(&product_name.to_be_bytes()) {
                Ok(product_name) => {
                    info!("Product name: {}", product_name);
                }
                Err(_) => {
                    info!("Product name: 0x{:010X} (invalid UTF-8)", product_name)
                }
            }
            info!("Product Revision: 0x{:02X}", cid.get_prv());
            info!("Product serial number: 0x{:08X}", cid.get_psn());
            info!("Manufacturing year: {}", cid.get_mdt().year());

            info!("Reading data");
            let before = Instant::now();
            let blocks_to_read = 20_480.min(capacity / 512) as u32;
            let success_count = command_18(&mut spi_bus, &mut cs, 0, blocks_to_read)
                .await
                .unwrap();
            // for i in 0..blocks_to_read {
            //     let mut buffer = [Default::default(); _];
            //     match command_17(&mut spi_bus, &mut cs, i, &mut buffer).await {
            //         Ok(()) => {
            //             // info!("Read block {}", i)
            //         }
            //         Err(spi_sd_card::Error::SpiBus(_)) | Err(spi_sd_card::Error::CsPin(_)) => {
            //             error!("[{}] SPI erorr", i);
            //         }
            //         Err(spi_sd_card::Error::BadR1(r1)) => {
            //             error!("[{}] Bad r1: 0b{:08b}", i, r1.bits());
            //         }
            //         Err(spi_sd_card::Error::BadData(data)) => {
            //             error!("[{}] Bad data: 0x{:02X}", i, data);
            //         }
            //         Err(spi_sd_card::Error::InvalidChecksum) => {
            //             error!("[{}] Invalid checksum", i);
            //         }
            //         _ => unreachable!(),
            //     };
            // }
            let after = Instant::now();
            info!(
                "Attempted to read {} blocks in {} ms. {} blocks were successfully read.",
                blocks_to_read,
                (after - before).as_millis(),
                success_count
            );

            // loop {
            //     info!("Checking that the SD card is still connected");
            //     let r2_byte_1 = command_13(&mut spi_bus, &mut cs).await.unwrap();
            //     if !r2_byte_1.is_empty() {
            //         error!("R2 contains an error flag");
            //         break;
            //     }
            //     info!("SD card is still connected");
            //     Timer::after_secs(2).await;
            // }
        }
        Err(spi_sd_card::Error::VoltageNotSupported) => {
            error!("Voltage (2.7V-3.6V) not supported");
        }
        Err(spi_sd_card::Error::BadR1(r1)) => {
            error!("R1 error: 0b{:08b}", r1.bits());
            todo!("Try to initialize a version 1 SD card");
            // info!("sending CMD58");
            // let ocr = command_58(&mut spi_bus, &mut cs).await.unwrap();
            // info!("OCR: 0b{:032b}", ocr.bits());
            // assert!(ocr.supports_3_3v());

            // loop {
            //     info!("sending CMD55");
            //     command_55(&mut spi_bus, &mut cs).await.unwrap();

            //     // Simulate talking to a different SPI device
            //     SpiBus::write(&mut spi_bus, &[0xFF; 1000]).await.unwrap();

            //     info!("sending ACMD41");
            //     let is_idle = command_a41(&mut spi_bus, &mut cs, false).await.unwrap();
            //     if is_idle {
            //         info!("SD card is not ready yet");

            //         // Simulate talking to a different SPI device
            //         SpiBus::write(&mut spi_bus, &[0xFF; 1000]).await.unwrap();
            //     } else {
            //         break;
            //     }
            // }
            // info!("SD card (version 1) is ready");
        }
        result => result.unwrap(),
    };
}
