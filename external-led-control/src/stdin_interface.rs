use std::{
    io::Error,
    sync::{Arc, RwLock},
    time::Duration,
};

use futures::prelude::*;

use crate::{action::Action, interface::Interface, stdin::get_stdin_stream};

pub struct StdinInterface {
    value: Arc<RwLock<bool>>,
}

impl StdinInterface {
    pub fn new(value: Arc<RwLock<bool>>) -> (Self, impl Stream<Item = Action>) {
        let (line_stream, _stop_reading_stdin) = get_stdin_stream(Duration::from_millis(10));
        let stream = line_stream
            .map(|byte| Ok::<[u8; 1], Error>([byte]))
            .into_async_read()
            .lines()
            .filter_map(|line| async {
                if let Ok(line) = line {
                    line.parse().map_or(None, |s| Some(s))
                } else {
                    None
                }
            });

        (Self { value }, stream)
    }
}

impl Interface for StdinInterface {
    fn notify_change(&mut self) {
        println!("New value: {}", *self.value.read().unwrap());
    }

    fn stop(self) {
        todo!()
    }
}
