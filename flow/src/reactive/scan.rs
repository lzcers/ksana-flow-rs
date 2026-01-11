use super::observable::{Observable, Observer, OperatorSubscription};
use std::marker::PhantomData;

pub struct ScanOperator<Source, F, Item, U> {
    pub upstream: Source,
    pub scan_fn: F,
    pub acc_value: U,
    pub _phantom: PhantomData<Item>,
}

impl<Item, Err, Source, F, U> Observable<U, Err> for ScanOperator<Source, F, Item, U>
where
    Source: Observable<Item, Err>,
    F: FnMut(U, Item) -> U + Send + 'static,
    U: Clone + Send + 'static,
{
    type Sub = OperatorSubscription<Source::Sub>;

    fn subscribe(self, observer: impl Observer<U, Err>) -> Self::Sub {
        let sub = self.upstream.subscribe(ScanObserver {
            downstream: observer,
            scan_fn: self.scan_fn,
            acc_value: self.acc_value,
        });
        OperatorSubscription { source_sub: sub }
    }
}

struct ScanObserver<D, F, U> {
    downstream: D,
    scan_fn: F,
    acc_value: U,
}

impl<Item, Err, D, F, U> Observer<Item, Err> for ScanObserver<D, F, U>
where
    D: Observer<U, Err>,
    F: FnMut(U, Item) -> U + Send + 'static,
    U: Clone + Send + 'static,
{
    fn on_next(&mut self, value: Item) {
        let new_value = (self.scan_fn)(self.acc_value.clone(), value);
        self.downstream.on_next(new_value.clone());
        self.acc_value = new_value;
    }

    fn on_error(&mut self, error: Err) {
        self.downstream.on_error(error);
    }

    fn on_completed(&mut self) {
        self.downstream.on_completed();
    }
}

#[cfg(test)]
mod tests {
    use super::super::observable::*;

    struct NumStream;
    struct NumSubscription;
    impl Subscription for NumSubscription {
        fn unsubscribe(self) {
            todo!()
        }
    }
    impl Observable<i32, ()> for NumStream {
        type Sub = NumSubscription;
        fn subscribe(self, mut observer: impl Observer<i32, ()>) -> Self::Sub {
            observer.on_next(1);
            observer.on_next(2);
            observer.on_next(3);
            observer.on_next(4);
            NumSubscription
        }
    }

    #[test]
    fn scan_operator_test() {
        let source = NumStream;
        let scan = source.scan(0, |acc, x| acc + x);
        let mut observer = scan.subscribe(|x| println!("{:?}", x));
    }
}
