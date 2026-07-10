//! `Observable` 到工作流 `TaskEvent` 的适配层。
//!
//! ReactiveStream 本身是一次性订阅描述；真正的生产任务在 Runner 收到
//! `TaskEvent::Stream` 后创建，并用额外的 `TaskGuard` 纳入完成条件。

use super::{TaskGuard, graph::NodeId, runner::TaskEvent};
use crate::{
    Context,
    reactive::observable::{Observable, Observer, Subscription},
};
use async_trait::async_trait;
use serde::Serialize;
use serde_json::Value;
use std::sync::Arc;
use tokio::{sync::mpsc, task::JoinHandle};

/// 一次性订阅闭包。调用后返回的 `Subscription` 负责取消底层生产任务。
pub type StreamSubscriptionFn = Box<
    dyn FnOnce(TaskGuard, mpsc::Sender<TaskEvent>, NodeId, Arc<Context>) -> Box<dyn Subscription>
        + Send,
>;
/// 节点输出携带的延迟订阅描述。
///
/// `subscribe` 是 `FnOnce`，一个 ReactiveStream 只能交给一个 Runner 消费一次。
pub struct ReactiveStream {
    pub subscribe: StreamSubscriptionFn,
}

/// 流结束时的聚合函数。
///
/// 使用该模式会在内存中保留全部 item；无限流或大流应使用增量聚合节点。
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

struct TaskSubscription {
    handle: JoinHandle<()>,
}

impl Subscription for TaskSubscription {
    /// 显式取消生产任务。仅丢弃 `TaskSubscription` 不会 abort Tokio JoinHandle。
    fn unsubscribe(self: Box<Self>) {
        self.handle.abort();
    }
}

impl ReactiveStream {
    /// 把 Observable 包装为逐项发送 `Next`、结束时发送空 `Completed` 的流。
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
                let handle = tokio::spawn(async move {
                    let _guard = guard;
                    let _sub = observable.subscribe(observer).await;
                });
                Box::new(TaskSubscription { handle })
            }),
        }
    }

    /// 包装 Observable，并在结束时把全部 item 交给 accumulator 生成最终输出。
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
                let handle = tokio::spawn(async move {
                    let _guard = guard;
                    let _sub = observable.subscribe(observer).await;
                });
                Box::new(TaskSubscription { handle })
            }),
        }
    }
}
