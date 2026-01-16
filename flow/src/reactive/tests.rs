use super::observable::*;
use async_trait::async_trait;
use std::time::Duration;
use tokio::time::sleep;

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

#[async_trait]
impl Observable<u32, ()> for NumStream {
    type Sub = NumSubscription;

    async fn subscribe(self, mut observer: impl Observer<u32, ()>) -> Self::Sub {
        let mut i = 0;
        loop {
            observer.on_next(i).await;
            i += 1;
            sleep(Duration::from_millis(300)).await;
            eprintln!("NumStream: {}", i);
            if i >= self.max {
                observer.on_completed().await;
                return NumSubscription;
            }
        }
    }
}
