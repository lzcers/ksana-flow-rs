use std::marker::PhantomData;

use super::observable::{Observable, Observer, OperatorSubscription};

pub struct MapObserver<D, F> {
    downstream: D,
    map_fn: F,
}

impl<T, U, E, D, F> Observer<T, E> for MapObserver<D, F>
where
    D: Observer<U, E>,
    F: FnMut(T) -> U,
{
    fn on_next(&mut self, value: T) {
        let mapped = (self.map_fn)(value);
        self.downstream.on_next(mapped)
    }
    fn on_error(self, error: E) {
        self.downstream.on_error(error);
    }
    fn on_completed(self) {
        self.downstream.on_completed();
    }
}

pub struct MapOperator<Source, MapFn, Item> {
    pub upstream: Source,
    pub map_fn: MapFn,
    pub _phantom: PhantomData<Item>,
}

impl<Source, MapFn, Item, Err, U> Observable<U, Err> for MapOperator<Source, MapFn, Item>
where
    Source: Observable<Item, Err>,
    MapFn: FnMut(Item) -> U,
{
    type Sub = OperatorSubscription<Source::Sub>;

    fn subscribe(self, observer: impl Observer<U, Err>) -> Self::Sub {
        let map_observer = MapObserver {
            downstream: observer,
            map_fn: self.map_fn,
        };
        let source_sub = self.upstream.subscribe(map_observer);
        OperatorSubscription { source_sub }
    }
}

#[cfg(test)]
mod tests {
    use super::super::observable::*;

    struct NumObservable {}
    struct NumObservableSub {}

    impl Subscription for NumObservableSub {
        fn unsubscribe(self) {}
    }

    impl Observable<i32, ()> for NumObservable {
        type Sub = NumObservableSub;
        fn subscribe(self, mut observer: impl Observer<i32, ()>) -> Self::Sub {
            observer.on_next(1);
            observer.on_next(2);
            observer.on_next(3);
            observer.on_next(4);
            NumObservableSub {}
        }
    }
    #[test]
    fn test_observable_subscribe() {
        let observable = NumObservable {};
        let sub = observable.subscribe(|v| println!("{v}"));
        sub.unsubscribe();
    }

    #[test]
    fn test_observable_map() {
        let observable = NumObservable {};
        let sub = observable
            .map(|v| v * 2)
            .map(|v| v * 2)
            .subscribe(|v| println!("{v}"));
        sub.unsubscribe();
    }
}
