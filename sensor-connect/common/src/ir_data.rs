use std::convert::TryInto;
use std::ops::Add;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Clone, Copy, Debug)]
pub struct IrData {
    pub is_receiving_light: bool,
    pub time: SystemTime,
}

impl IrData {
    pub fn to_bytes(&self) -> [u8; 9] {
        let bool = [u8::from(self.is_receiving_light)];
        let time = u64::try_from(self.time.duration_since(UNIX_EPOCH).unwrap().as_nanos())
            .unwrap()
            .to_be_bytes();

        bool.into_iter()
            .chain(time.into_iter())
            .collect::<Vec<u8>>()
            .try_into()
            .unwrap()
    }

    pub fn from_bytes(bytes: [u8; 9]) -> Self {
        Self {
            is_receiving_light: bytes[0] != 0,
            time: SystemTime::UNIX_EPOCH.add(Duration::from_nanos(u64::from_be_bytes(
                bytes[1..].try_into().unwrap(),
            ))),
        }
    }
}