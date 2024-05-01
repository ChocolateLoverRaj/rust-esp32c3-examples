use std::ops::Add;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Clone, Copy, Debug)]
pub struct DistanceData {
    pub distance: u16,
    pub time: SystemTime,
}

impl DistanceData {
    pub fn to_bytes(&self) -> [u8; 10] {
        let distance = self.distance.to_be_bytes();
        let time = u64::try_from(self.time.duration_since(UNIX_EPOCH).unwrap().as_nanos())
            .unwrap()
            .to_be_bytes();

        distance
            .into_iter()
            .chain(time.into_iter())
            .collect::<Vec<u8>>()
            .try_into()
            .unwrap()
    }

    pub fn from_bytes(bytes: [u8; 10]) -> Self {
        Self {
            distance: u16::from_be_bytes(bytes[0..2].try_into().unwrap()),
            time: SystemTime::UNIX_EPOCH.add(Duration::from_nanos(u64::from_be_bytes(
                bytes[2..].try_into().unwrap(),
            ))),
        }
    }
}
