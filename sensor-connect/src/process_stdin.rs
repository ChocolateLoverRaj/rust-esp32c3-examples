use std::{sync::Arc, time::Duration};

use esp32_nimble::{utilities::mutex::Mutex, BLECharacteristic};
use futures::{AsyncBufReadExt, StreamExt, TryStreamExt};
use log::warn;
use serde::{Deserialize, Serialize};

use crate::{
    info::INFO, short_name_characteristic::ShortNameCharacteristic, stdin::get_stdin_stream,
    validate_short_name::validate_short_name,
};

pub async fn process_stdin(
    short_name_characteristic: &mut ShortNameCharacteristic,
    passkey_characteristic: &Arc<Mutex<BLECharacteristic>>,
    set_passkey: &Arc<std::sync::Mutex<impl Fn(u32)>>,
) {
    let (stdin_stream, _stop_stdin_stream) = get_stdin_stream(Duration::from_millis(10));
    let mut usb_lines_stream = stdin_stream
        .map(|byte| Ok::<[u8; 1], std::io::Error>([byte]))
        .into_async_read()
        .lines();

    #[derive(Serialize, Deserialize)]
    enum GetSet<T> {
        Get,
        Set(T),
    }

    #[derive(Serialize, Deserialize)]
    enum Command {
        Info,
        ShortName(GetSet<String>),
        Passkey(GetSet<u32>),
    }

    loop {
        let line = usb_lines_stream.next().await.unwrap().unwrap();

        let command: serde_json::Result<Command> = serde_json::from_str(&line);
        match command {
            Ok(command) => match command {
                Command::Info => {
                    let info_str = serde_json::to_string(&INFO).unwrap();
                    println!("{}", info_str);
                }
                Command::ShortName(sub) => match sub {
                    GetSet::Get => {
                        println!("{:?}", short_name_characteristic.get());
                    }
                    GetSet::Set(short_name) => match validate_short_name(&short_name) {
                        Ok(_) => {
                            short_name_characteristic.set(&short_name);
                            println!();
                        }
                        Err(e) => {
                            warn!("{}", e);
                        }
                    },
                },
                Command::Passkey(sub) => match sub {
                    GetSet::Get => {
                        let passkey = u32::from_be_bytes(
                            <&[u8] as TryInto<[u8; 4]>>::try_into(
                                passkey_characteristic.lock().value_mut().value(),
                            )
                            .unwrap(),
                        );
                        println!("{}", serde_json::to_string(&passkey).unwrap());
                    }
                    GetSet::Set(passkey) => set_passkey.lock().unwrap()(passkey),
                },
            },
            Err(e) => {
                println!("Invalid command: {:#?}", e);
            }
        }
    }
}
