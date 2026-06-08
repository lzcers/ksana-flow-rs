use async_trait::async_trait;
use std::marker::PhantomData;

use super::observable::{Observable, Observer, OperatorSubscription};

pub struct MapObserver<D, F> {
    downstream: D,
    map_fn: F,
}

#[async_trait]
impl<T, U, E, D, F> Observer<T, E> for MapObserver<D, F>
where
    D: Observer<U, E>,
    F: FnMut(T) -> U + Send + 'static,
    U: Send + 'static,
    T: Send + 'static,
    E: Send + 'static,
{
    async fn on_next(&mut self, value: T) {
        let mapped = (self.map_fn)(value);
        self.downstream.on_next(mapped).await
    }
    async fn on_error(&mut self, error: E) {
        self.downstream.on_error(error).await;
    }
    async fn on_completed(&mut self) {
        self.downstream.on_completed().await;
    }
}

pub struct MapOperator<Source, MapFn, Item> {
    pub upstream: Source,
    pub map_fn: MapFn,
    pub _phantom: PhantomData<Item>,
}

#[async_trait]
impl<Source, MapFn, Item, Err, U> Observable<U, Err> for MapOperator<Source, MapFn, Item>
where
    Source: Observable<Item, Err> + Send + Sync + 'static,
    MapFn: FnMut(Item) -> U + Send + 'static,
    U: Send + 'static,
    Item: Send + 'static,
    Err: Send + 'static,
{
    type Sub = OperatorSubscription<Source::Sub>;

    async fn subscribe(self, observer: impl Observer<U, Err>) -> Self::Sub {
        let map_observer = MapObserver {
            downstream: observer,
            map_fn: self.map_fn,
        };
        let source_sub = self.upstream.subscribe(map_observer).await;
        OperatorSubscription { source_sub }
    }
}

#[cfg(test)]
mod tests {
    use super::super::observable::*;
    use async_trait::async_trait;

    struct NumObservable {}
    struct NumObservableSub {}

    impl Subscription for NumObservableSub {
        fn unsubscribe(self: Box<Self>) {}
    }

    #[async_trait]
    impl Observable<i32, ()> for NumObservable {
        type Sub = NumObservableSub;
        async fn subscribe(self, mut observer: impl Observer<i32, ()>) -> Self::Sub {
            observer.on_next(1).await;
            observer.on_next(2).await;
            observer.on_next(3).await;
            observer.on_next(4).await;
            NumObservableSub {}
        }
    }
    #[tokio::test]
    async fn test_observable_subscribe() {
        let observable = NumObservable {};
        let sub = observable.subscribe(|v| println!("{v}")).await;
        Box::new(sub).unsubscribe();
    }

    #[tokio::test]
    async fn test_observable_map() {
        let observable = NumObservable {};
        let sub = observable
            .map(|v| v * 2)
            .map(|v| v * 2)
            .subscribe(|v| println!("{v}"))
            .await;
        Box::new(sub).unsubscribe();
    }
}
