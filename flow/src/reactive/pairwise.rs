use super::observable::{Observable, Observer, OperatorSubscription};

pub struct PairwiseOperator<Upstream, Item> {
    pub upstream: Upstream,
    pub prev_item: Option<Item>,
}

impl<Item, Err, Upstream> Observable<(Option<Item>, Item), Err> for PairwiseOperator<Upstream, Item>
where
    Upstream: Observable<Item, Err>,
    Item: Clone,
{
    type Sub = OperatorSubscription<Upstream::Sub>;

    fn subscribe(self, observer: impl Observer<(Option<Item>, Item), Err>) -> Self::Sub {
        let sub = self.upstream.subscribe(PairwiseObserver {
            downstream: observer,
            prev_item: None,
        });
        OperatorSubscription { source_sub: sub }
    }
}

pub struct PairwiseObserver<Item, Downstream> {
    pub downstream: Downstream,
    pub prev_item: Option<Item>,
}

impl<Item, Err, Downstream> Observer<Item, Err> for PairwiseObserver<Item, Downstream>
where
    Downstream: Observer<(Option<Item>, Item), Err>,
    Item: Clone,
{
    fn on_next(&mut self, value: Item) {
        let prev_value = self.prev_item.take();
        self.prev_item = Some(value.clone());
        self.downstream.on_next((prev_value, value));
    }

    fn on_error(self, error: Err) {
        self.downstream.on_error(error);
    }

    fn on_completed(self) {
        self.downstream.on_completed();
    }
}

#[cfg(test)]

mod tests {

    use super::super::{
        observable::{Observable, ObservableExt},
        tests::NumStream,
    };

    #[test]
    fn test_pairwise() {
        NumStream::new(100).pairwise().subscribe(|(prev, cur)| {
            eprintln!("pairwise: {:?}", (prev, cur));
        });
    }
}
