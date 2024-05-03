use std::rc::Rc;
use std::sync::{Arc, Mutex};
use futures_signals::signal::{Mutable, Signal};

pub struct Subscription {
    subscribable3: Subscribable3
}

impl Drop for Subscription {
    fn drop(&mut self) {
        self.subscribable3.unsubscribe();
    }
}

#[derive(Default, Clone)]
pub struct Subscribable3 {
    mutable: Mutable<bool>,
    subscriber_count: Arc<Mutex<usize>>
}

impl Subscribable3 {
    fn unsubscribe(&self) {
        let mut subscriber_count = self.subscriber_count.lock().unwrap();
        *subscriber_count -= 1;
        if *subscriber_count == 0 {
            self.mutable.set(false);
        }
    }

    pub fn subscribe(&self) -> Subscription {
        let mut subscriber_count = self.subscriber_count.lock().unwrap();
        if *subscriber_count == 0 {
            self.mutable.set(true);
        }
        *subscriber_count += 1;
        Subscription {
            subscribable3: self.to_owned()
        }
    }

    pub fn signal(&self) -> impl Signal<Item=bool> {
        self.mutable.signal()
    }

    pub fn get(&self) -> bool {
        self.mutable.get()
    }
}