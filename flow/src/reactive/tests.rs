use super::observable::*;
use std::{thread::sleep, time::Duration};
pub struct NumStream {
    max: u32,
}

impl NumStream {
    pub fn new(max: u32) -> Self {
        Self { max }
    }
}

pub struct NumSubscription;

impl Subscription for NumSubscription {
    fn unsubscribe(self) {}
}

impl Observable<u32, ()> for NumStream {
    type Sub = NumSubscription;

    fn subscribe(self, mut observer: impl Observer<u32, ()>) -> Self::Sub {
        let mut i = 0;
        loop {
            observer.on_next(i);
            i += 1;
            sleep(Duration::from_millis(300));
            eprintln!("NumStream: {}", i);
            if i >= self.max {
                observer.on_completed();
                return NumSubscription;
            }
        }
    }
}
