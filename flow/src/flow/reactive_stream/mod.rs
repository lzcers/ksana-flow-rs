//! 异步流到工作流 `TaskEvent` 的适配层。
//!
//! `ReactiveStream` 本身是一次性消费描述；真正的生产任务在 Runner 收到
//! `TaskEvent::Stream` 后创建，并用额外的 `TaskGuard` 纳入完成条件。

use super::{TaskGuard, graph::NodeId, runner::TaskEvent};
use crate::Context;
use futures::{Stream, StreamExt};
use serde::Serialize;
use serde_json::Value;
use std::{future::Future, sync::Arc};
use tokio::{sync::mpsc, task::JoinHandle};

/// 一次性流启动闭包。返回的任务句柄由 Runner 管理和取消。
pub type StreamStartFn = Box<
    dyn FnOnce(TaskGuard, mpsc::Sender<TaskEvent>, NodeId, Arc<Context>) -> JoinHandle<()> + Send,
>;

/// 节点输出携带的延迟启动描述。
///
/// `start` 是 `FnOnce`，一个 `ReactiveStream` 只能交给一个 Runner 消费一次。
pub struct ReactiveStream {
    pub start: StreamStartFn,
}

/// 流结束时的聚合函数。
///
/// 使用该模式会在内存中保留全部 item；无限流或大流应使用增量聚合节点。
type AccumulatorFn<T> = Box<dyn Fn(Vec<T>) -> Option<Value> + Send>;

impl ReactiveStream {
    /// 创建一个由 Runner 延迟启动的流任务。
    pub fn new<F, Fut>(producer: F) -> Self
    where
        F: FnOnce(TaskGuard, mpsc::Sender<TaskEvent>, NodeId, Arc<Context>) -> Fut + Send + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        Self {
            start: Box::new(move |guard, tx, node_id, ctx| {
                tokio::spawn(producer(guard, tx, node_id, ctx))
            }),
        }
    }

    /// 把异步流包装为逐项发送 `Next`、结束时发送空 `Completed` 的工作流流。
    pub fn from_stream<T, E, S>(stream: S) -> Self
    where
        T: Serialize + Send + 'static,
        E: Send + 'static,
        S: Stream<Item = Result<T, E>> + Send + 'static,
    {
        Self::from_stream_inner(stream, None)
    }

    /// 包装异步流，并在结束时把全部 item 交给 accumulator 生成最终输出。
    pub fn from_stream_with_accumulator<T, E, S, F>(stream: S, accumulator: F) -> Self
    where
        T: Serialize + Send + 'static,
        E: Send + 'static,
        S: Stream<Item = Result<T, E>> + Send + 'static,
        F: Fn(Vec<T>) -> Option<Value> + Send + 'static,
    {
        Self::from_stream_inner(stream, Some(Box::new(accumulator)))
    }

    fn from_stream_inner<T, E, S>(stream: S, accumulator: Option<AccumulatorFn<T>>) -> Self
    where
        T: Serialize + Send + 'static,
        E: Send + 'static,
        S: Stream<Item = Result<T, E>> + Send + 'static,
    {
        Self::new(move |guard, tx, node_id, _ctx| async move {
            let _guard = guard;
            let mut stream = Box::pin(stream);
            let mut acc_buffer = Vec::new();

            while let Some(result) = stream.next().await {
                let item = match result {
                    Ok(item) => item,
                    Err(_) => {
                        let _ = tx
                            .send(TaskEvent::Error(node_id, "Stream error".to_string()))
                            .await;
                        return;
                    }
                };
                let value = match serde_json::to_value(&item) {
                    Ok(value) => value,
                    Err(error) => {
                        let _ = tx
                            .send(TaskEvent::Error(
                                node_id,
                                format!("Stream serialize error: {error}"),
                            ))
                            .await;
                        return;
                    }
                };

                if accumulator.is_some() {
                    acc_buffer.push(item);
                }
                if tx
                    .send(TaskEvent::Next(node_id.clone(), value))
                    .await
                    .is_err()
                {
                    return;
                }
            }

            let final_output = accumulator.and_then(|accumulate| accumulate(acc_buffer));
            let _ = tx.send(TaskEvent::Completed(node_id, final_output)).await;
        })
    }
}

#[cfg(test)]
mod tests;
