use std::{
    borrow::BorrowMut,
    sync::{Arc, Mutex},
    time::Duration,
};

use futures::{
    channel::mpsc::{channel, Receiver, UnboundedReceiver},
    join, AsyncBufReadExt, StreamExt, TryStreamExt,
};
use log::warn;
use serde::{Deserialize, Serialize};

use crate::{
    ble_on_characteristic::BleOnCharacteristic,
    info::INFO,
    ir_sensor::{IrData, IrSensor, IrSubscribable},
    passkey_characteristic::PasskeyCharacteristic,
    short_name_characteristic::ShortNameCharacteristic,
    stdin::get_stdin_stream,
    validate_short_name::validate_short_name,
    vl53l0x_sensor::{DistanceData, DistanceSubscribable},
};

pub struct IrInput {
    pub subscribable: IrSubscribable,
    pub ir_sensor: Arc<Mutex<IrSensor>>,
}

pub async fn process_stdin(
    short_name_characteristic: &mut ShortNameCharacteristic,
    mut short_name_change_receiver: Receiver<()>,
    passkey_characteristic: &mut PasskeyCharacteristic,
    mut passkey_change_receiver: Receiver<()>,
    ble_on_characteristic: &mut BleOnCharacteristic,
    mut ble_on_change_receiver: Receiver<()>,
    mut ir: Option<IrInput>,
    mut distance_subscribable: Option<DistanceSubscribable>,
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
    enum Subscribe {
        Ir,
        Distance,
    }

    #[derive(Serialize, Deserialize)]
    enum Command {
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
    enum Message {
        ShortNameChange,
        PasskeyChange,
        BleOnChange,
    }

    #[derive(Serialize, Deserialize)]
    struct Capabilities {
        ir: bool,
        distance: bool,
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
                ble_on_change_receiver.next().await.unwrap();
                println!("{}", serde_json::to_string(&Message::BleOnChange).unwrap());
            }
        },
        async {
            let mut ir_subscription_id = None::<usize>;
            let (mut ir_tx, mut ir_rx) = channel::<UnboundedReceiver<IrData>>(0);

            let mut distance_subscription_id = None::<usize>;
            let (mut distance_tx, mut distance_rx) = channel::<UnboundedReceiver<DistanceData>>(0);

            join!(
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
                                    GetSet::Set(short_name) => {
                                        match validate_short_name(&short_name) {
                                            Ok(_) => {
                                                short_name_characteristic
                                                    .set_externally(&short_name);
                                                println!();
                                            }
                                            Err(e) => {
                                                warn!("{}", e);
                                            }
                                        }
                                    }
                                },
                                Command::Passkey(sub) => match sub {
                                    GetSet::Get => {
                                        println!(
                                            "{}",
                                            serde_json::to_string(&passkey_characteristic.get())
                                                .unwrap()
                                        );
                                    }
                                    GetSet::Set(passkey) => {
                                        passkey_characteristic.set_externally(passkey)
                                    }
                                },
                                Command::BleOn(sub) => match sub {
                                    GetSet::Get => {
                                        println!(
                                            "{}",
                                            serde_json::to_string(&ble_on_characteristic.get())
                                                .unwrap()
                                        );
                                    }
                                    GetSet::Set(on) => ble_on_characteristic.set_external(on),
                                },
                                Command::Subscribe(subscribe) => match subscribe {
                                    Subscribe::Ir => match ir.borrow_mut() {
                                        Some(ir) => {
                                            let (rx, id) = ir.subscribable.subscribe();
                                            ir_subscription_id = Some(id);
                                            ir_tx.try_send(rx).unwrap();
                                        }
                                        None => {
                                            warn!("No IR Sensor connected");
                                        }
                                    },
                                    Subscribe::Distance => match distance_subscribable.borrow_mut()
                                    {
                                        Some(distance_subscribable) => {
                                            warn!("Subscribing to distance");
                                            let (rx, id) = distance_subscribable.subscribe();
                                            distance_subscription_id = Some(id);
                                            distance_tx.try_send(rx).unwrap();
                                        }
                                        None => {
                                            warn!("No distance sensor connected");
                                        }
                                    },
                                },
                                Command::Unsubscribe(subscribe) => match subscribe {
                                    Subscribe::Ir => match ir_subscription_id {
                                        Some(id) => match ir.borrow_mut() {
                                            Some(ir) => {
                                                ir.subscribable.unsubscribe(id);
                                                ir_subscription_id = None;
                                            }
                                            None => {
                                                warn!("No IR sensor connected");
                                            }
                                        },
                                        None => {
                                            warn!("Cannot unsubscribe because currently not subscribed");
                                        }
                                    },
                                    Subscribe::Distance => match distance_subscription_id {
                                        Some(id) => match distance_subscribable.borrow_mut() {
                                            Some(distance_subscribable) => {
                                                warn!("Unsubscribing from distance");

                                                distance_subscribable.unsubscribe(id);
                                                distance_subscription_id = None;
                                            }
                                            None => {
                                                warn!("No distance sensor connected");
                                            }
                                        },
                                        None => {
                                            warn!("Cannot unsubscribe because currently not subscribed");
                                        }
                                    },
                                },
                                Command::ReadIr => {
                                    match ir.borrow_mut() {
                                        Some(ir) => {
                                            println!("Aquiring lock");
                                            // FIXME: While the ir loop is running, the pin is locked because it is waiting for an edge, which requires write access
                                            println!(
                                                "{}",
                                                ir.ir_sensor
                                                    .try_lock()
                                                    .unwrap()
                                                    .turn_on_and_check_is_receiving_light()
                                            );
                                        }
                                        None => {
                                            warn!("IR not connected");
                                        }
                                    }
                                }
                                Command::GetCapabilities => {
                                    println!(
                                        "{}",
                                        serde_json::to_string(&Capabilities {
                                            distance: distance_subscribable.is_some(),
                                            ir: ir.is_some()
                                        })
                                        .unwrap()
                                    );
                                }
                            },
                            Err(e) => {
                                println!("Invalid command: {:#?}", e);
                            }
                        }
                    }
                },
                async {
                    loop {
                        let mut rx = ir_rx.next().await.unwrap();
                        loop {
                            match rx.next().await {
                                Some(value) => {
                                    println!("New value: {:#?}", value);
                                    // TODO: Send updates in an easy to parse way
                                }
                                None => break,
                            };
                        }
                    }
                },
                async {
                    loop {
                        let mut rx = distance_rx.next().await.unwrap();
                        loop {
                            match rx.next().await {
                                Some(value) => {
                                    println!("New distance value: {:#?}", value);
                                }
                                None => break,
                            }
                        }
                    }
                }
            );
        }
    );
}
