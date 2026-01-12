use super::observable::{Observable, Observer};
use async_trait::async_trait;
use std::time::Duration;
use tokio::time::sleep;

pub struct DelayOperator<Upstream> {
    pub upstream: Upstream,
    pub delay: Duration,
}

#[async_trait]
impl<Upstream, Item, Err> Observable<Item, Err> for DelayOperator<Upstream>
where
    Upstream: Observable<Item, Err> + Send + Sync,
    Item: Send + Sync + 'static,
    Err: Send + Sync + 'static,
{
    type Sub = Upstream::Sub;

    async fn subscribe(self, observer: impl Observer<Item, Err> + 'static) -> Self::Sub {
        self.upstream
            .subscribe(DelayObserver {
                downstream: observer,
                delay: self.delay,
            })
            .await
    }
}

pub struct DelayObserver<Downstream> {
    downstream: Downstream,
    delay: Duration,
}

#[async_trait]
impl<Item, Err, Downstream> Observer<Item, Err> for DelayObserver<Downstream>
where
    Downstream: Observer<Item, Err> + Send,
    Item: Send + 'static,
    Err: Send + 'static,
{
    async fn on_next(&mut self, value: Item) {
        sleep(self.delay).await;
        self.downstream.on_next(value).await;
    }

    async fn on_error(&mut self, error: Err) {
        self.downstream.on_error(error).await;
    }

    async fn on_completed(&mut self) {
        self.downstream.on_completed().await;
    }
}
