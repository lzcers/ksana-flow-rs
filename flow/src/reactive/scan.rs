use super::observable::{Observable, Observer, OperatorSubscription};
use async_trait::async_trait;
use std::marker::PhantomData;

pub struct ScanOperator<Source, F, Item, U> {
    pub upstream: Source,
    pub scan_fn: F,
    pub acc_value: U,
    pub _phantom: PhantomData<Item>,
}

#[async_trait]
impl<Item, Err, Source, F, U> Observable<U, Err> for ScanOperator<Source, F, Item, U>
where
    Source: Observable<Item, Err> + Send + 'static,
    F: FnMut(U, Item) -> U + Send + 'static,
    U: Clone + Send + 'static,
    Item: Send + 'static,
    Err: Send + 'static,
{
    type Sub = OperatorSubscription<Source::Sub>;

    async fn subscribe(self, observer: impl Observer<U, Err>) -> Self::Sub {
        let sub = self
            .upstream
            .subscribe(ScanObserver {
                downstream: observer,
                scan_fn: self.scan_fn,
                acc_value: self.acc_value,
            })
            .await;
        OperatorSubscription { source_sub: sub }
    }
}

struct ScanObserver<D, F, U> {
    downstream: D,
    scan_fn: F,
    acc_value: U,
}

#[async_trait]
impl<Item, Err, D, F, U> Observer<Item, Err> for ScanObserver<D, F, U>
where
    D: Observer<U, Err>,
    F: FnMut(U, Item) -> U + Send + 'static,
    U: Clone + Send + 'static,
    Item: Send + 'static,
    Err: Send + 'static,
{
    async fn on_next(&mut self, value: Item) {
        let new_value = (self.scan_fn)(self.acc_value.clone(), value);
        self.downstream.on_next(new_value.clone()).await;
        self.acc_value = new_value;
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
    use super::super::observable::*;
    use async_trait::async_trait;

    struct NumStream;
    struct NumSubscription;
    impl Subscription for NumSubscription {
        fn unsubscribe(self) {}
    }
    #[async_trait]
    impl Observable<i32, ()> for NumStream {
        type Sub = NumSubscription;
        async fn subscribe(self, mut observer: impl Observer<i32, ()>) -> Self::Sub {
            observer.on_next(1).await;
            observer.on_next(2).await;
            observer.on_next(3).await;
            observer.on_next(4).await;
            NumSubscription
        }
    }

    #[tokio::test]
    async fn scan_operator_test() {
        let source = NumStream;
        let scan = source.scan(0, |acc, x| acc + x);
        let _sub = scan.subscribe(|x| println!("{:?}", x)).await;
    }
}
