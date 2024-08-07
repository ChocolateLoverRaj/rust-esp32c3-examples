use std::time::Duration;

use anyhow::{anyhow, Context};
use ir_remote::ir_signal::{IrPacket, IrSignal, RemoteType, Repeat};
use postcard::to_allocvec;
use tokio::{
    fs::{File, OpenOptions},
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader, Lines},
    time::{sleep, sleep_until, Instant},
};

pub struct SoundSystem {
    file: File,
    lines: Lines<BufReader<File>>,
    last_sent: Option<Instant>,
}

impl SoundSystem {
    pub async fn open() -> anyhow::Result<Self> {
        let w_file = OpenOptions::new()
            .write(true)
            .read(false)
            .open("/dev/ttyACM0")
            .await?;
        Ok(Self {
            file: w_file,
            lines: BufReader::new(
                OpenOptions::new()
                    .read(true)
                    .write(false)
                    .open("/dev/ttyACM0")
                    .await?,
            )
            .lines(),
            last_sent: Default::default(),
        })
    }

    async fn press_button(&mut self, button: u8) -> anyhow::Result<()> {
        // Don't send >1 signal within 500ms
        if let Some(last_sent) = self.last_sent.take() {
            sleep_until(last_sent + Duration::from_millis(500)).await;
        }
        let repeat = Some(Repeat {
            times: 2,
            duration_between: Duration::from_secs_f64(0.027116677),
        });
        let receiver_id = 0xA55A;
        let remote_type = RemoteType::Generic;
        self.file
            .write_all(&to_allocvec(&IrSignal {
                packet: IrPacket {
                    remote_type,
                    receiver_id,
                    button,
                },
                repeat,
            })?)
            .await
            .context("write_all error")?;
        self.file.flush().await.context("flush error")?;
        self.last_sent = Some(Instant::now());
        loop {
            let line = self.lines.next_line().await?.ok_or(anyhow!("No reply"))?;
            println!("Line: {:?}", line);
            if line == "Sent signal" {
                break;
            }
        }
        Ok(())
    }

    /// Turns on the sound system and sets input to TV
    pub async fn turn_on(&mut self) -> anyhow::Result<()> {
        self.press_button(0x38)
            .await
            .context("Error sending power button")?;
        // It takes some time to turn on
        sleep(Duration::from_secs(2)).await;
        self.press_button(0x30)
            .await
            .context("Error sending TV button")?;
        Ok(())
    }

    pub async fn turn_off(&mut self) -> anyhow::Result<()> {
        self.press_button(0x38)
            .await
            .context("Error sending power button")?;
        Ok(())
    }
}
