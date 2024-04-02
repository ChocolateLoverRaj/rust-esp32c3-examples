use rand::RngCore;
use serde::{Deserialize, Serialize};

pub const SERVICE_UUID: &str = "c5f93147-b051-4201-bb59-ff8f18db9876";
pub const SHORT_NAME_UUID: &str = "ec67e1ac-cdd0-44bd-9c03-aebc64968b68";
pub const PASSKEY_UUID: &str = "f0650e70-58ff-4b69-ab99-5d61c6db7e75";
pub const BLE_ON_UUID: &str = "3c534064-8559-45e8-84d1-761d1c5ef438";

pub mod validate_short_name;

#[derive(Serialize, Deserialize, Debug)]
pub enum GetSet<T> {
    Get,
    Set(T),
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Subscribe {
    Ir,
    Distance,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum CommandData {
    Info,
    ShortName(GetSet<String>),
    Passkey(GetSet<u32>),
    BleOn(GetSet<bool>),
    Subscribe(Subscribe),
    Unsubscribe(Subscribe),
    ReadIr,
    GetCapabilities,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MessageToEsp {
    pub id: u32,
    pub command: CommandData,
}

impl MessageToEsp {
    pub fn new(command: CommandData) -> Self {
        Self {
            id: rand::thread_rng().next_u32(),
            command,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum Message {
    ShortNameChange,
    PasskeyChange,
    BleOnChange,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Capabilities {
    pub ir: bool,
    pub distance: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ResponseData {
    GetInfo(Info),
    GetShortName(String),
    GetPasskey(u32),
    GetBleOn(bool),
    Complete,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Response {
    pub id: u32,
    pub data: ResponseData,
}
// impl Response {
//     pub fn new(data: ResponseData) -> Self {
//         Self {
//             id: rand::thread_rng().next_u32(),
//             data,
//         }
//     }
// }

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum MessageFromEsp {
    Response(Response),
    Event(Message),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Info {
    pub name: String,
    pub version: String,
    pub homepage: String,
    pub repository: String,
    pub authors: String,
}
