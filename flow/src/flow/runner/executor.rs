use super::task_guard::TaskGuard;
use super::utils::send_task_event;
use crate::{AnyNode, Context, Input, Output, TaskEvent};
use futures::FutureExt;
use std::{panic::AssertUnwindSafe, sync::Arc};
use tokio::sync::{RwLock, mpsc};
use tracing::{debug, info};
// 接收调度器发送来的任务进行执行
pub struct Executor;

impl Executor {
    pub fn exec(
        _guard: TaskGuard,
        node_id: String,
        node: Arc<RwLock<dyn AnyNode>>,
        input: Input,
        // 节点执行所需的上下文
        ctx: Arc<Context>,
        // 向调度器发送执行结果
        task_sender: mpsc::Sender<TaskEvent>,
    ) {
        tokio::spawn(async move {
            let _keep_alive = _guard; // Force capture
            let mut node = node.write().await;

            info!(node_id = %node_id, "Node tasks start");
            let result = AssertUnwindSafe(node.run(ctx.as_ref(), &input))
                .catch_unwind()
                .await;

            let output: Result<Output, String> = match result {
                Ok(res) => res,
                Err(panic_err) => {
                    let msg = if let Some(s) = panic_err.downcast_ref::<&str>() {
                        format!("Panic: {}", s)
                    } else if let Some(s) = panic_err.downcast_ref::<String>() {
                        format!("Panic: {}", s)
                    } else {
                        "Panic: unknown error".to_string()
                    };
                    Err(msg)
                }
            };

            debug!(node_id = %node_id, "Node logic executed");

            match output {
                Ok(out) => {
                    let is_stream = out.is_stream();
                    if is_stream {
                        match out.into_stream() {
                            Some(stream) => {
                                send_task_event(
                                    &task_sender,
                                    TaskEvent::Stream(node_id, stream.subscribe),
                                )
                                .await;
                            }
                            None => {
                                send_task_event(
                                    &task_sender,
                                    TaskEvent::Error(node_id, "Missing stream".to_string()),
                                )
                                .await;
                            }
                        }
                    } else {
                        send_task_event(&task_sender, TaskEvent::Completed(node_id, out.into_value()))
                            .await;
                    }
                }
                Err(e) => {
                    send_task_event(&task_sender, TaskEvent::Error(node_id, e)).await;
                }
            }
        });
    }
}
