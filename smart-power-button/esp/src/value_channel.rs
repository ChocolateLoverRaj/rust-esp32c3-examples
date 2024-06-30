use std::sync::Arc;

use parking_lot::{RwLock, RwLockReadGuard, RwLockUpgradableReadGuard};
use tokio::sync::broadcast;
use tokio::sync::broadcast::Sender;

pub struct ValueSender<T> {
    value: Arc<RwLock<T>>,
    sender: Sender<()>,
}

impl<T> ValueSender<T> {
    pub fn get(&self) -> RwLockReadGuard<T> {
        self.value.read()
    }

    pub async fn update(&self, value: T) {
        *self.value.write() = value;
        let _ = self.sender.send(());
    }
}

impl<T: PartialEq> ValueSender<T> {
    pub async fn update_if_changed(&self, new_value: T) {
        let value = self.value.upgradable_read();
        if *value != new_value {
            let mut value = RwLockUpgradableReadGuard::upgrade(value);
            *value = new_value;
            let _ = self.sender.send(());
        }
    }
}

#[derive(Clone)]
pub struct ValueReceiver<T> {
    value: Arc<RwLock<T>>,
    sender: Sender<()>,
}

impl<T: Clone> ValueReceiver<T> {
    pub fn get(&self) -> T {
        self.value.read().clone()
    }

    pub async fn until_change(&mut self) {
        self.sender.subscribe().recv().await.unwrap();
    }
}

pub fn value_channel<T>(initial_value: T) -> (ValueSender<T>, ValueReceiver<T>) {
    let value = Arc::new(RwLock::new(initial_value));
    let (sender, _) = broadcast::channel(16);
    (ValueSender {
        value: value.clone(),
        sender: sender.clone(),
    }, ValueReceiver {
        value: value.clone(),
        sender: sender.clone(),
    })
}