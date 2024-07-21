use anyhow::anyhow;
use esp_idf_svc::nvs::{EspDefaultNvs, EspDefaultNvsPartition, EspNvs};
use postcard::{from_bytes, to_allocvec};

use crate::value_channel::{value_channel, ValueReceiver, ValueSender};

pub struct BluetoothWakeupDevices {
    nvs: EspDefaultNvs,
    value_sender: ValueSender<Vec<[u8; 6]>>,
}

const TAG: &str = "devices";

impl BluetoothWakeupDevices {
    pub fn new(nvs: EspDefaultNvsPartition) -> anyhow::Result<(Self, ValueReceiver<Vec<[u8; 6]>>)> {
        let nvs = EspNvs::new(nvs, "wakeup_devices", true)?;
        let (value_sender, value_receiver) = value_channel({
            match nvs.blob_len(TAG)? {
                Some(len) => {
                    let mut buffer = vec![Default::default(); len];
                    let bytes = nvs
                        .get_blob(TAG, &mut buffer)?
                        .ok_or(anyhow!("None blob"))?;
                    from_bytes(bytes)?
                }
                None => Default::default(),
            }
        });
        Ok((Self { nvs, value_sender }, value_receiver))
    }

    pub async fn set(&mut self, devices: Vec<[u8; 6]>) -> anyhow::Result<()> {
        self.nvs.set_blob(TAG, &to_allocvec(&devices)?)?;
        self.value_sender.update(devices).await;
        Ok(())
    }
}
