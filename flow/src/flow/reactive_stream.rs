use async_trait::async_trait;
use core::fmt;
use std::{any::Any, sync::Arc};

use tokio::sync::mpsc;

use crate::{
    flow::{
        event::TaskEvent,
        graph::{Context, NodeId},
        sendable_any::SendableAny,
    },
    reactive::observable::{Observable, Observer, Subscription},
};

pub type StreamSubscriptionFn = Box<
    dyn FnOnce(mpsc::Sender<TaskEvent>, NodeId, Arc<Context>) -> Box<dyn Subscription>
        + Send
        + Sync,
>;
pub struct ReactiveStream<T = ()> {
    pub subscribe: StreamSubscriptionFn,
    _marker: std::marker::PhantomData<T>,
}

struct RunnerObserver {
    tx: mpsc::Sender<TaskEvent>,
    node_id: String,
}

#[async_trait]
impl<T: SendableAny, E: Send + 'static> Observer<T, E> for RunnerObserver {
    async fn on_next(&mut self, value: T) {
        // 直接使用异步 send，Tokio 会在通道满时自动挂起当前协程
        // 这实现了完美的异步背压
        let _ = self
            .tx
            .send(TaskEvent::Next(self.node_id.clone(), Box::new(value)))
            .await;
    }
    async fn on_error(&mut self, _error: E) {
        let _ = self
            .tx
            .send(TaskEvent::Error(
                self.node_id.clone(),
                "Stream error".to_string(),
            ))
            .await;
    }
    async fn on_completed(&mut self) {
        let _ = self
            .tx
            .send(TaskEvent::Completed(self.node_id.clone(), None))
            .await;
    }
}

struct EmptySubscription;
impl Subscription for EmptySubscription {
    fn unsubscribe(self) {}
}

impl<T> ReactiveStream<T> {
    pub fn from_observable<E, O>(observable: O) -> Self
    where
        T: SendableAny + 'static,
        E: Send + 'static,
        O: Observable<T, E> + Send + Sync + 'static,
    {
        Self {
            subscribe: Box::new(move |tx, node_id, _ctx| {
                let observer = RunnerObserver { tx, node_id };
                tokio::spawn(async move {
                    let _sub = observable.subscribe(observer).await;
                });
                Box::new(EmptySubscription)
            }),
            _marker: std::marker::PhantomData,
        }
    }
}

impl<T> fmt::Debug for ReactiveStream<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ReactiveStream")
            .field("type", &std::any::type_name::<T>())
            .finish()
    }
}

impl<T: 'static + Send + Clone> SendableAny for ReactiveStream<T> {
    fn clone_box(&self) -> Box<dyn SendableAny> {
        panic!("ReactiveStream cannot be cloned. It should only be used once in the flow.");
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }
    fn type_name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }
    fn is_stream(&self) -> bool {
        true
    }
    fn into_stream_subscriber(
        self: Box<Self>,
    ) -> Result<StreamSubscriptionFn, Box<dyn SendableAny>> {
        Ok(self.subscribe)
    }
}
