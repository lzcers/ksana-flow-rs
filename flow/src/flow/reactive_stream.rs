use async_trait::async_trait;
use std::sync::Arc;

use tokio::sync::mpsc;

use super::{
    graph::{Context, NodeId},
    {TaskGuard, event::TaskEvent},
};
use crate::reactive::observable::{Observable, Observer, Subscription};

use serde::Serialize;
use serde_json::Value;

pub type StreamSubscriptionFn = Box<
    dyn FnOnce(TaskGuard, mpsc::Sender<TaskEvent>, NodeId, Arc<Context>) -> Box<dyn Subscription>
        + Send,
>;
pub struct ReactiveStream {
    pub subscribe: StreamSubscriptionFn,
}

pub type AccumulatorFn<T> = Box<dyn Fn(Vec<T>) -> Option<Value> + Send>;

struct RunnerObserver<T> {
    tx: mpsc::Sender<TaskEvent>,
    node_id: String,
    acc_buffer: Vec<T>,
    accumulator: Option<AccumulatorFn<T>>,
}

#[async_trait]
impl<T: Serialize + Send + Clone + 'static, E: Send + 'static> Observer<T, E>
    for RunnerObserver<T>
{
    async fn on_next(&mut self, value: T) {
        if self.accumulator.is_some() {
            self.acc_buffer.push(value.clone());
        }

        // 直接使用异步 send，Tokio 会在通道满时自动挂起当前协程
        // 这实现了完美的异步背压
        match serde_json::to_value(&value) {
            Ok(v) => {
                let _ = self.tx.send(TaskEvent::Next(self.node_id.clone(), v)).await;
            }
            Err(e) => {
                let _ = self
                    .tx
                    .send(TaskEvent::Error(
                        self.node_id.clone(),
                        format!("Stream serialize error: {}", e),
                    ))
                    .await;
            }
        }
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

impl ReactiveStream {
    pub fn from_observable<T, E, O>(observable: O) -> Self
    where
        T: Serialize + Send + Clone + 'static,
        E: Send + 'static,
        O: Observable<T, E> + Send + 'static,
    {
        Self {
            subscribe: Box::new(move |guard: TaskGuard, tx, node_id, _ctx| {
                let observer = RunnerObserver {
                    tx,
                    node_id,
                    acc_buffer: Vec::new(),
                    accumulator: None,
                };
                tokio::spawn(async move {
                    let _guard = guard;
                    let _sub = observable.subscribe(observer).await;
                });
                Box::new(EmptySubscription)
            }),
        }
    }

    pub fn from_observable_with_accumulator<T, E, O, F>(observable: O, accumulator: F) -> Self
    where
        T: Serialize + Send + Clone + 'static,
        E: Send + 'static,
        O: Observable<T, E> + Send + 'static,
        F: Fn(Vec<T>) -> Option<Value> + Send + 'static,
    {
        Self {
            subscribe: Box::new(move |guard: TaskGuard, tx, node_id, _ctx| {
                let observer = RunnerObserver {
                    tx,
                    node_id,
                    acc_buffer: Vec::new(),
                    accumulator: Some(Box::new(accumulator)),
                };
                tokio::spawn(async move {
                    let _guard = guard;
                    let _sub = observable.subscribe(observer).await;
                });
                Box::new(EmptySubscription)
            }),
        }
    }
}
