use super::observable::{Observable, Observer, OperatorSubscription};

pub struct FilterObserver<D, F> {
    downstream: D,
    filter_fn: F,
}

impl<T, E, D, F> Observer<T, E> for FilterObserver<D, F>
where
    D: Observer<T, E>,
    F: FnMut(&T) -> bool + Send + 'static,
    T: Send + 'static,
{
    fn on_next(&mut self, value: T) {
        if (self.filter_fn)(&value) {
            self.downstream.on_next(value)
        }
    }
    fn on_error(&mut self, error: E) {
        self.downstream.on_error(error);
    }
    fn on_completed(&mut self) {
        self.downstream.on_completed();
    }
}

pub struct FilterOperator<Source, FilterFn> {
    pub upstream: Source,
    pub filter_fn: FilterFn,
}

impl<Source, FilterFn, Item, Err> Observable<Item, Err> for FilterOperator<Source, FilterFn>
where
    Source: Observable<Item, Err>,
    FilterFn: FnMut(&Item) -> bool + Send + 'static,
    Item: Send + 'static,
{
    type Sub = OperatorSubscription<Source::Sub>;

    fn subscribe(self, observer: impl Observer<Item, Err>) -> Self::Sub {
        let filter_observer = FilterObserver {
            downstream: observer,
            filter_fn: self.filter_fn,
        };
        let source_sub = self.upstream.subscribe(filter_observer);
        OperatorSubscription { source_sub }
    }
}

#[cfg(test)]
mod tests {
    use super::super::observable::*;
    use super::super::tests::NumStream;

    #[test]
    fn test_filter_operator() {
        let sub = NumStream::new(10)
            .filter(|v| v % 2 == 0)
            .subscribe(|v| println!("{v}"));
        sub.unsubscribe();
    }
}
