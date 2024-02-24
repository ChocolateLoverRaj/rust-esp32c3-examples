use std::time::SystemTime;

use common::validate_short_name::SHORT_NAME_MAX_LENGTH;
use esp_idf_svc::nvs::{EspNvs, NvsDefault};
use random::Source;

pub const NVS_TAG_SHORT_NAME: &str = "short_name";
const NAME_RANDOM_BYTES: usize = 1;
const INITIAL_NAME: &str = "OpenSensor";

pub fn get_short_name(nvs: &mut EspNvs<NvsDefault>) -> String {
    // Add 1 cuz it needs an extra character for \0 (which we will trim)
    let mut buf = [0u8; SHORT_NAME_MAX_LENGTH + 1];
    let stored_name = nvs.get_str(NVS_TAG_SHORT_NAME, &mut buf).unwrap();
    match stored_name {
        Some(stored_name) => stored_name.trim_end_matches(char::from(0)).to_owned(),
        None => {
            let seed = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_nanos() as u64;
            let mut source = random::default(seed);
            let bytes =
                hex::encode_upper(source.iter().take(NAME_RANDOM_BYTES).collect::<Vec<u8>>());
            let name = format!("{} {}", INITIAL_NAME, bytes);
            nvs.set_str(NVS_TAG_SHORT_NAME, name.as_str()).unwrap();
            name
        }
    }
}
