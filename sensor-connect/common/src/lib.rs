use serde::{Deserialize, Serialize};

pub const SERVICE_UUID: &str = "c5f93147-b051-4201-bb59-ff8f18db9876";
pub const SHORT_NAME_UUID: &str = "ec67e1ac-cdd0-44bd-9c03-aebc64968b68";

#[derive(Serialize, Deserialize)]
pub enum GetSet<T> {
    Get,
    Set(T),
}

#[derive(Serialize, Deserialize)]
pub enum Subscribe {
    Ir,
    Distance,
}

#[derive(Serialize, Deserialize)]
pub enum Command {
    Info,
    ShortName(GetSet<String>),
    Passkey(GetSet<u32>),
    BleOn(GetSet<bool>),
    Subscribe(Subscribe),
    Unsubscribe(Subscribe),
    ReadIr,
    GetCapabilities,
}

#[derive(Serialize, Deserialize)]
pub enum Message {
    ShortNameChange,
    PasskeyChange,
    BleOnChange,
}

#[derive(Serialize, Deserialize)]
pub struct Capabilities {
    ir: bool,
    distance: bool,
}
