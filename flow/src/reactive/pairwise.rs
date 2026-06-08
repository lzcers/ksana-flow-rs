use super::observable::{Observable, Observer, OperatorSubscription};
use async_trait::async_trait;

pub struct PairwiseOperator<Upstream, Item> {
    pub upstream: Upstream,
    pub prev_item: Option<Item>,
}

#[async_trait]
impl<Item, Err, Upstream> Observable<(Option<Item>, Item), Err> for PairwiseOperator<Upstream, Item>
where
    Upstream: Observable<Item, Err> + Send + 'static,
    Item: Clone + Send + 'static,
    Err: Send + 'static,
{
    type Sub = OperatorSubscription<Upstream::Sub>;

    async fn subscribe(self, observer: impl Observer<(Option<Item>, Item), Err>) -> Self::Sub {
        let sub = self
            .upstream
            .subscribe(PairwiseObserver {
                downstream: observer,
                prev_item: None,
            })
            .await;
        OperatorSubscription { source_sub: sub }
    }
}

pub struct PairwiseObserver<Item, Downstream> {
    pub downstream: Downstream,
    pub prev_item: Option<Item>,
}

#[async_trait]
impl<Item, Err, Downstream> Observer<Item, Err> for PairwiseObserver<Item, Downstream>
where
    Downstream: Observer<(Option<Item>, Item), Err>,
    Item: Clone + Send + 'static,
    Err: Send + 'static,
{
    async fn on_next(&mut self, value: Item) {
        let prev_value = self.prev_item.take();
        self.prev_item = Some(value.clone());
        self.downstream.on_next((prev_value, value)).await;
    }

    async fn on_error(&mut self, error: Err) {
        self.downstream.on_error(error).await;
    }

    async fn on_completed(&mut self) {
        self.downstream.on_completed().await;
    }
}

#[cfg(test)]
mod tests {
    use super::super::{
        observable::{Observable, ObservableExt},
        tests::NumStream,
    };

    #[tokio::test]
    async fn test_pairwise() {
        NumStream::new(10)
            .pairwise()
            .subscribe(|(prev, cur)| {
                eprintln!("pairwise: {:?}", (prev, cur));
            })
            .await;
    }
}
