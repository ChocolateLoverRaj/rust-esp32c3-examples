use std::time::Duration;

use ir_remote::ir_signal::{IrPacket, IrSignal, RemoteType, Repeat};
use postcard::to_allocvec;
use tokio::{
    fs::{File, OpenOptions},
    io::AsyncWriteExt,
    time::sleep,
};

pub struct SoundSystem {
    file: File,
}

impl SoundSystem {
    pub async fn open() -> anyhow::Result<Self> {
        let file = OpenOptions::new()
            .write(true)
            .read(false)
            .open("/dev/ttyACM0")
            .await?;
        Ok(Self { file })
    }

    async fn press_button(&mut self, button: u8) -> anyhow::Result<()> {
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
            .await?;
        self.file.flush().await?;
        Ok(())
    }

    /// Turns on the sound system and sets input to TV
    pub async fn turn_on(&mut self) -> anyhow::Result<()> {
        self.press_button(0x38).await?;
        sleep(Duration::from_secs(1)).await;
        self.press_button(0x30).await?;
        Ok(())
    }

    pub async fn turn_off(&mut self) -> anyhow::Result<()> {
        self.press_button(0x38).await?;
        Ok(())
    }
}
