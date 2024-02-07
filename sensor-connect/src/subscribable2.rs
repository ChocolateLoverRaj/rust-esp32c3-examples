use std::sync::{Arc, Mutex};

use futures::channel::mpsc::{
    channel, unbounded, Receiver, Sender, UnboundedReceiver, UnboundedSender,
};
use slab::Slab;

pub trait InternalSubscribable<T> {
    fn on_subscribe(&mut self, update: Sender<T>);
    fn on_unsubscribe(&mut self);
}

#[derive(Clone)]
pub struct Subscribable2<T: Copy + Send> {
    tx: Arc<Mutex<Sender<()>>>,
    subscribers: Arc<Mutex<Slab<UnboundedSender<T>>>>,
}

impl<T: Copy + Send + 'static> Subscribable2<T> {
    pub fn new() -> (Self, Receiver<()>) {
        let (tx, rx) = channel::<()>(0);
        (
            Self {
                tx: Arc::new(Mutex::new(tx)),
                subscribers: Arc::new(Mutex::new(Slab::new())),
            },
            rx,
        )
    }

    pub fn subscribe(&mut self) -> (UnboundedReceiver<T>, usize) {
        let (tx, rx) = unbounded::<T>();
        let mut subscribers = self.subscribers.lock().unwrap();
        let previous_len = subscribers.len();
        let id = subscribers.insert(tx);
        if previous_len == 0 {
            self.tx.lock().unwrap().try_send(()).unwrap();
        }
        (rx, id)
    }

    pub fn unsubscribe(&mut self, id: usize) {
        let mut subscribers = self.subscribers.lock().unwrap();
        subscribers.remove(id);
        if subscribers.len() == 0 {
            self.tx.lock().unwrap().try_send(()).unwrap();
        }
    }

    pub fn update(&mut self, message: T) {
        for (_, subscriber) in self.subscribers.lock().unwrap().iter_mut() {
            subscriber.unbounded_send(message).unwrap();
        }
    }
}
