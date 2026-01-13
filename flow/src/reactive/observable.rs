use async_trait::async_trait;
use std::marker::PhantomData;

use crate::delay::DelayOperator;

use super::filter::FilterOperator;

pub use super::pairwise::PairwiseOperator;

pub use super::map::MapOperator;
pub use super::scan::ScanOperator;

pub trait Subscription: Send + Sync {
    fn unsubscribe(self);
}

#[async_trait]
pub trait Observable<Item, Err> {
    type Sub: Subscription;
    async fn subscribe(self, observer: impl Observer<Item, Err> + 'static) -> Self::Sub;
}

#[async_trait]
pub trait Observer<Item, Err>: Send + 'static {
    async fn on_next(&mut self, value: Item);
    async fn on_error(&mut self, error: Err);
    async fn on_completed(&mut self);
}

#[async_trait]
impl<Item, Err, F> Observer<Item, Err> for F
where
    Item: Send + 'static,
    Err: Send + 'static,
    F: FnMut(Item) + Send + 'static,
{
    async fn on_next(&mut self, value: Item) {
        (self)(value);
    }
    async fn on_error(&mut self, _err: Err) {
        eprintln!("Encounter error in FnMut Observer");
    }
    async fn on_completed(&mut self) {}
}

pub struct OperatorSubscription<S> {
    pub source_sub: S,
}

impl<S: Subscription> Subscription for OperatorSubscription<S> {
    fn unsubscribe(self) {
        self.source_sub.unsubscribe();
    }
}

pub struct VecSubscription;

impl Subscription for VecSubscription {
    fn unsubscribe(self) {}
}

#[async_trait]
impl<Item: Send + 'static> Observable<Item, ()> for Vec<Item> {
    type Sub = VecSubscription;

    async fn subscribe(self, mut observer: impl Observer<Item, ()>) -> Self::Sub {
        for item in self {
            observer.on_next(item).await;
        }
        observer.on_completed().await;
        VecSubscription
    }
}

pub trait ObservableExt<Item, Err>: Sized + Observable<Item, Err> {
    fn map<U, F: FnMut(Item) -> U>(self, map_fn: F) -> MapOperator<Self, F, Item> {
        MapOperator {
            upstream: self,
            map_fn,
            _phantom: PhantomData,
        }
    }
    fn filter<F: FnMut(&Item) -> bool>(self, filter_fn: F) -> FilterOperator<Self, F> {
        FilterOperator {
            upstream: self,
            filter_fn,
        }
    }
    fn scan<U, F: FnMut(U, Item) -> U>(
        self,
        init: U,
        scan_fn: F,
    ) -> ScanOperator<Self, F, Item, U> {
        ScanOperator {
            upstream: self,
            scan_fn,
            acc_value: init,
            _phantom: PhantomData,
        }
    }
    fn pairwise(self) -> PairwiseOperator<Self, Item> {
        PairwiseOperator {
            upstream: self,
            prev_item: None,
        }
    }
    fn delay(self, duration: std::time::Duration) -> DelayOperator<Self> {
        DelayOperator {
            upstream: self,
            delay: duration,
        }
    }
}

impl<Item, Err, T> ObservableExt<Item, Err> for T where T: Observable<Item, Err> {}

#[cfg(test)]
mod tests {

    #[test]
    fn test_vec_observable() {}
}
