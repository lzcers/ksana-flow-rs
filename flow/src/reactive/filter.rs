use async_trait::async_trait;

use super::observable::{Observable, Observer, OperatorSubscription};

pub struct FilterObserver<D, F> {
    downstream: D,
    filter_fn: F,
}

#[async_trait]
impl<T, E, D, F> Observer<T, E> for FilterObserver<D, F>
where
    D: Observer<T, E>,
    F: FnMut(&T) -> bool + Send + 'static,
    T: Send + 'static,
    E: Send + 'static,
{
    async fn on_next(&mut self, value: T) {
        if (self.filter_fn)(&value) {
            self.downstream.on_next(value).await
        }
    }
    async fn on_error(&mut self, error: E) {
        self.downstream.on_error(error).await;
    }
    async fn on_completed(&mut self) {
        self.downstream.on_completed().await;
    }
}

pub struct FilterOperator<Source, FilterFn> {
    pub upstream: Source,
    pub filter_fn: FilterFn,
}

#[async_trait]
impl<Source, FilterFn, Item, Err> Observable<Item, Err> for FilterOperator<Source, FilterFn>
where
    Source: Observable<Item, Err> + Send + Sync + 'static,
    FilterFn: FnMut(&Item) -> bool + Send + 'static,
    Item: Send + 'static,
    Err: Send + 'static,
{
    type Sub = OperatorSubscription<Source::Sub>;

    async fn subscribe(self, observer: impl Observer<Item, Err>) -> Self::Sub {
        let filter_observer = FilterObserver {
            downstream: observer,
            filter_fn: self.filter_fn,
        };
        let source_sub = self.upstream.subscribe(filter_observer).await;
        OperatorSubscription { source_sub }
    }
}

#[cfg(test)]
mod tests {
    use super::super::observable::*;
    use super::super::tests::NumStream;

    #[tokio::test]
    async fn test_filter_operator() {
        let sub = NumStream::new(10)
            .filter(|v| v % 2 == 0)
            .subscribe(|v| println!("{v}"))
            .await;
        Box::new(sub).unsubscribe();
    }
}
