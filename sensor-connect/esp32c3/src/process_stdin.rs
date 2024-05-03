use std::{
    borrow::{Borrow, BorrowMut},
    sync::{Arc, Mutex},
    time::Duration,
};
use std::time::SystemTime;

use futures::{
    AsyncBufReadExt,
    channel::mpsc::{channel, Receiver, UnboundedReceiver}, join, StreamExt, TryStreamExt,
};
use futures::future::join3;
use log::{error, info, warn};
use serde::Serialize;

use common::{
    Capabilities, CommandData, GetSet, Message, MessageFromEsp,
    MessageToEsp, Response, ResponseData, Subscribe, validate_short_name::validate_short_name,
    ir_data::IrData,
};
use common::distance_data::DistanceData;

use crate::{
    ble_on_characteristic::BleOnCharacteristic,
    info::get_info,
    ir_sensor::{IrSensor, IrSubscribable},
    passkey_characteristic::PasskeyCharacteristic,
    short_name_characteristic::ShortNameCharacteristic,
    stdin::get_stdin_stream,
};
use crate::subscribable3::{Subscribable3, Subscription};
use crate::vl53l0x_sensor::DistanceSensor;

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
    distance: Option<(Arc<Subscribable3>, async_channel::Receiver<DistanceData>, Arc<futures::lock::Mutex<DistanceSensor>>)>,
) {
    let (stdin_stream, _stop_stdin_stream) = get_stdin_stream(Duration::from_millis(10));
    let mut usb_lines_stream = stdin_stream
        .map(|byte| Ok::<[u8; 1], std::io::Error>([byte]))
        .into_async_read()
        .lines();

    let passkey_change_fut = async {
        loop {
            passkey_change_receiver.next().await.unwrap();
            println!(
                "{}",
                serde_json::to_string(&MessageFromEsp::Event(Message::PasskeyChange)).unwrap()
            );
        }
    };

    let ble_on_change_fut = async {
        loop {
            ble_on_change_receiver.next().await.unwrap();
            println!("{}", serde_json::to_string(&MessageFromEsp::Event(Message::BleOnChange)).unwrap());
        }
    };

    let mut distance_subscription = futures::lock::Mutex::new(None::<(Subscription, Option<DistanceData>)>);

    let commands_fut = async {
        let mut ir_subscription_id = None::<usize>;
        let (mut ir_tx, mut ir_rx) = channel::<UnboundedReceiver<IrData>>(0);

        let commands_fut = async {
            loop {
                let line = usb_lines_stream.next().await.unwrap().unwrap();

                let command: serde_json::Result<MessageToEsp> = serde_json::from_str(&line);
                match command {
                    Ok(command) => match command.command {
                        CommandData::Info => {
                            println!(
                                "{}",
                                serde_json::to_string(&MessageFromEsp::Response(
                                    Response {
                                        id: command.id,
                                        data: ResponseData::GetInfo(get_info()),
                                    }
                                ))
                                    .unwrap()
                            );
                        }
                        CommandData::ShortName(sub) => match sub {
                            GetSet::Get => {
                                println!(
                                    "{}",
                                    serde_json::to_string(&MessageFromEsp::Response(
                                        Response {
                                            id: command.id,
                                            data: ResponseData::GetShortName(
                                                short_name_characteristic.get()
                                            ),
                                        }
                                    ))
                                        .unwrap()
                                );
                            }
                            GetSet::Set(short_name) => {
                                match validate_short_name(&short_name) {
                                    Ok(_) => {
                                        short_name_characteristic
                                            .set_externally(&short_name);
                                        println!(
                                            "{}",
                                            serde_json::to_string(
                                                &MessageFromEsp::Response(Response {
                                                    id: command.id,
                                                    data: ResponseData::Complete,
                                                })
                                            )
                                                .unwrap()
                                        );
                                    }
                                    Err(e) => {
                                        warn!("{}", e);
                                    }
                                }
                            }
                        },
                        CommandData::Passkey(sub) => match sub {
                            GetSet::Get => {
                                println!(
                                    "{}",
                                    serde_json::to_string(&MessageFromEsp::Response(Response {
                                        id: command.id,
                                        data: ResponseData::GetPasskey(passkey_characteristic.get()),
                                    }))
                                        .unwrap()
                                );
                            }
                            GetSet::Set(passkey) => {
                                passkey_characteristic.set_externally(passkey)
                            }
                        },
                        CommandData::BleOn(sub) => match sub {
                            GetSet::Get => {
                                println!(
                                    "{}",
                                    serde_json::to_string(&MessageFromEsp::Response(Response {
                                        id: command.id,
                                        data: ResponseData::GetBleOn(ble_on_characteristic.get()),
                                    })).unwrap()
                                );
                            }
                            GetSet::Set(on) => ble_on_characteristic.set_external(on),
                        },
                        CommandData::ReadDistance => {
                            info!("Hi");
                            match &distance {
                                Some((_, _, distance_sensor)) => {
                                    let distance_subscription = distance_subscription.lock().await;
                                    let distance_data = match distance_subscription.as_ref() {
                                        Some((subscription, distance_data)) => {
                                            // FIXME: Don't unwrap
                                            distance_data.unwrap()
                                        }
                                        None => {
                                            info!("Getting lock");
                                            let mut lock = distance_sensor.lock().await;
                                            info!("Got lock");
                                            let distance = lock.vl53l0x.read_range_single_millimeters_blocking().unwrap();
                                            info!("read distance");
                                            let distance_data = DistanceData {
                                                distance,
                                                time: SystemTime::now(),
                                            };
                                            distance_data
                                        }
                                    };

                                    println!(
                                        "{}",
                                        serde_json::to_string(&MessageFromEsp::Response(Response {
                                            id: command.id,
                                            data: ResponseData::GetDistance(distance_data),
                                        })).unwrap()
                                    );
                                }
                                None => {
                                    error!("No distance sensor");
                                }
                            }
                        }
                        CommandData::Subscribe(subscribe) => match subscribe {
                            Subscribe::Ir => match ir.borrow_mut() {
                                Some(ir) => match ir_subscription_id {
                                    None => {
                                        let (rx, id) = ir.subscribable.subscribe();
                                        ir_subscription_id = Some(id);
                                        ir_tx.try_send(rx).unwrap();
                                    }
                                    Some(_) => {
                                        warn!("Already subscribed to ir");
                                    }
                                },
                                None => {
                                    warn!("No IR Sensor connected");
                                }
                            },
                            Subscribe::Distance => match distance.borrow()
                            {
                                Some(distance_subscribable) => {
                                    match distance_subscription.lock().await.is_none() {
                                        true => {
                                            *distance_subscription.lock().await = Some((distance_subscribable.0.subscribe(), None));
                                        }
                                        false => {
                                            warn!("Already subscribed to distance");
                                        }
                                    }
                                    // {"id":0,"command":{"ReadDistance":null}}
                                }
                                None => {
                                    warn!("No distance sensor connected");
                                }
                            },
                        },
                        CommandData::Unsubscribe(subscribe) => match subscribe {
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
                            Subscribe::Distance => match distance_subscription.lock().await.is_some() {
                                true => {
                                    *distance_subscription.lock().await = None;
                                }
                                false => {
                                    warn!("Cannot unsubscribe because currently not subscribed");
                                }
                            },
                        },
                        CommandData::ReadIr => {
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
                        CommandData::GetCapabilities => {
                            println!(
                                "{}",
                                serde_json::to_string(&MessageFromEsp::Response(Response {
                                    id: command.id,
                                    data: ResponseData::GetCapabilities(Capabilities {
                                        distance: distance.is_some(),
                                        ir: ir.is_some(),
                                    }),
                                })).unwrap()
                            );
                        }
                    },
                    Err(e) => {
                        println!("Invalid command: {:#?}", e);
                    }
                }
            }
        };

        join3(
            commands_fut,
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
                if let Some((_subscribable, rx, distance_sensor)) = &distance {
                    loop {
                        let distance = rx.recv().await.unwrap();
                        distance_subscription.lock().await.as_mut().unwrap().1 = Some(distance);
                        println!(
                            "{}",
                            serde_json::to_string(&MessageFromEsp::Event(Message::DistanceChange)).unwrap()
                        );
                    }
                }
            },
            // async {
            //     loop {
            //         let mut rx = distance_rx.next().await.unwrap();
            //         loop {
            //             match rx.next().await {
            //                 Some(value) => {
            //                     println!("New distance value: {:#?}", value);
            //                 }
            //                 None => break,
            //             }
            //         }
            //     }
            // }
        ).await;
    };

    join!(
        async {
            loop {
                short_name_change_receiver.next().await.unwrap();
                println!(
                    "{}",
                    serde_json::to_string(&MessageFromEsp::Event(Message::ShortNameChange))
                        .unwrap()
                );
            }
        },
        passkey_change_fut,
        ble_on_change_fut,
        commands_fut
    );
}
