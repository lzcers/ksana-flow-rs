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

pub type StreamSubscriptionFn =
    Box<dyn FnOnce(mpsc::Sender<TaskEvent>, NodeId, Arc<Context>) -> Box<dyn Subscription> + Send>;
pub struct ReactiveStream<T = ()> {
    pub subscribe: StreamSubscriptionFn,
    _marker: std::marker::PhantomData<T>,
}

pub type AccumulatorFn<T> = Box<dyn Fn(Vec<T>) -> Option<Box<dyn SendableAny>> + Send>;

struct RunnerObserver<T> {
    tx: mpsc::Sender<TaskEvent>,
    node_id: String,
    acc_buffer: Vec<T>,
    accumulator: Option<AccumulatorFn<T>>,
}

#[async_trait]
impl<T: SendableAny + Clone, E: Send + 'static> Observer<T, E> for RunnerObserver<T> {
    async fn on_next(&mut self, value: T) {
        if self.accumulator.is_some() {
            self.acc_buffer.push(value.clone());
        }

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
        let final_output = if let Some(acc_fn) = &self.accumulator {
            let buffer = std::mem::take(&mut self.acc_buffer);
            acc_fn(buffer)
        } else {
            None
        };

        let _ = self
            .tx
            .send(TaskEvent::Completed(self.node_id.clone(), final_output))
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
        T: SendableAny + Clone + 'static,
        E: Send + 'static,
        O: Observable<T, E> + Send + 'static,
    {
        Self {
            subscribe: Box::new(move |tx, node_id, _ctx| {
                let observer = RunnerObserver {
                    tx,
                    node_id,
                    acc_buffer: Vec::new(),
                    accumulator: None,
                };
                tokio::spawn(async move {
                    let _sub = observable.subscribe(observer).await;
                });
                Box::new(EmptySubscription)
            }),
            _marker: std::marker::PhantomData,
        }
    }

    pub fn from_observable_with_accumulator<E, O, F>(observable: O, accumulator: F) -> Self
    where
        T: SendableAny + Clone + 'static,
        E: Send + 'static,
        O: Observable<T, E> + Send + 'static,
        F: Fn(Vec<T>) -> Option<Box<dyn SendableAny>> + Send + 'static,
    {
        Self {
            subscribe: Box::new(move |tx, node_id, _ctx| {
                let observer = RunnerObserver {
                    tx,
                    node_id,
                    acc_buffer: Vec::new(),
                    accumulator: Some(Box::new(accumulator)),
                };
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
