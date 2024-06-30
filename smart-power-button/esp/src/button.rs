use std::ops::DerefMut;
use std::sync::Arc;
use std::time::Duration;

use esp_idf_svc::hal::gpio::{Level, Output, OutputPin, PinDriver};
use esp_idf_svc::sys::EspError;
use tokio::sync::{Mutex, RwLock};
use tokio::sync::broadcast::{channel, Sender};
use tokio::time::sleep;

pub struct Button<T: OutputPin> {
    pin: Arc<Mutex<PinDriver<'static, T, Output>>>,
    sender: Sender<()>,
    is_pressed: Arc<RwLock<bool>>,
}

impl<T: OutputPin> Clone for Button<T> {
    fn clone(&self) -> Self {
        Self {
            pin: self.pin.clone(),
            sender: self.sender.clone(),
            is_pressed: self.is_pressed.clone(),
        }
    }
}

impl<T: OutputPin> Button<T> {
    pub fn new(pin: T) -> Result<Self, EspError> {
        let mut button = PinDriver::output(pin)?;
        button.set_high()?;
        Ok(Self {
            pin: Arc::new(Mutex::new(button)),
            sender: channel(16).0,
            is_pressed: Arc::new(RwLock::new(false)),
        })
    }

    pub async fn is_pressed(&self) -> bool {
        *self.is_pressed.read().await
    }

    async fn set_and_broadcast(&self, pin: &mut PinDriver<'static, T, Output>, level: Level) {
        pin.set_level(level).unwrap();
        *self.is_pressed.write().await = level == Level::Low;
        self.sender.send(()).unwrap();
    }

    async fn press(&self, duration: Duration) {
        let mut pin = self.pin.lock().await;
        self.set_and_broadcast(pin.deref_mut(), Level::Low).await;
        sleep(duration).await;
        self.set_and_broadcast(pin.deref_mut(), Level::High).await;
    }

    pub async fn short_press(&self) {
        self.press(Duration::from_millis(500)).await
    }

    pub async fn long_press(&self) {
        self.press(Duration::from_secs(6)).await
    }

    pub async fn until_change(&self) {
        self.sender.subscribe().recv().await.unwrap();
    }

    // pub fn try_lock(&self) -> Result<ButtonLock<T>, TryLockError> {
    //     let pin = self.pin.try_lock()?;
    //     Ok(ButtonLock {
    //         pin,
    //         sender: &self.sender
    //     })
    // }
}

// pub struct ButtonLock<'a, T: InputPin + OutputPin> {
//     pin: MutexGuard<'a, PinDriver<'static, T, InputOutput>>,
//     sender: &'a Sender<()>,
// }
//
// impl<'a, T: InputPin + OutputPin> ButtonLock<'a, T> {
//     async fn press(&mut self, duration: Duration) {
//         self.set_and_broadcast(self.pin.deref_mut(), Level::Low);
//         sleep(duration).await;
//         self.set_and_broadcast(self.pin.deref_mut(), Level::High);
//     }
//
//     pub async fn long_press(&mut self) {
//         self.press(Duration::from_secs(6)).await
//     }
// }

