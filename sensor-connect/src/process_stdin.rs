use std::time::Duration;

use futures::{channel::mpsc::Receiver, join, AsyncBufReadExt, StreamExt, TryStreamExt};
use log::warn;
use serde::{Deserialize, Serialize};

use crate::{
    info::INFO, passkey_characteristic::PasskeyCharacteristic,
    short_name_characteristic::ShortNameCharacteristic, stdin::get_stdin_stream,
    validate_short_name::validate_short_name,
};

pub async fn process_stdin(
    short_name_characteristic: &mut ShortNameCharacteristic,
    mut short_name_change_receiver: Receiver<()>,
    passkey_characteristic: &mut PasskeyCharacteristic,
    mut passkey_change_receiver: Receiver<()>,
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

    #[derive(Serialize, Deserialize)]
    enum Message {
        ShortNameChange,
        PasskeyChange,
    }

    join!(
        async {
            loop {
                short_name_change_receiver.next().await.unwrap();
                println!(
                    "{}",
                    serde_json::to_string(&Message::ShortNameChange).unwrap()
                );
            }
        },
        async {
            loop {
                passkey_change_receiver.next().await.unwrap();
                println!(
                    "{}",
                    serde_json::to_string(&Message::PasskeyChange).unwrap()
                );
            }
        },
        async {
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
                                    short_name_characteristic.set_externally(&short_name);
                                    println!();
                                }
                                Err(e) => {
                                    warn!("{}", e);
                                }
                            },
                        },
                        Command::Passkey(sub) => match sub {
                            GetSet::Get => {
                                println!(
                                    "{}",
                                    serde_json::to_string(&passkey_characteristic.get()).unwrap()
                                );
                            }
                            GetSet::Set(passkey) => passkey_characteristic.set_externally(passkey),
                        },
                    },
                    Err(e) => {
                        println!("Invalid command: {:#?}", e);
                    }
                }
            }
        }
    );
}
