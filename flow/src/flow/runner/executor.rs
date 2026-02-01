use super::{task_guard::TaskGuard, utils::send_task_event};
use crate::{
    Context,
    flow::{
        AnyNode, TaskEvent,
        graph::{Input, Output},
    },
};
use futures::FutureExt;
use std::{panic::AssertUnwindSafe, sync::Arc};
use tokio::sync::{RwLock, Semaphore, mpsc};
use tracing::{debug, info};

pub struct Executor {
    semaphore: Option<Arc<Semaphore>>,
    runtime_ctx: Arc<Context>,
}

impl Executor {
    pub fn new() -> Self {
        Self {
            runtime_ctx: Arc::new(Context::new()),
            semaphore: None,
        }
    }

    pub fn set_max_concurrency(&mut self, max: usize) {
        let max = max.max(1);
        self.semaphore = Some(Arc::new(Semaphore::new(max)));
    }

    pub fn clear_max_concurrency(&mut self) {
        self.semaphore = None;
    }

    pub fn get_context(&self) -> Arc<Context> {
        self.runtime_ctx.clone()
    }
    pub fn get_context_ref(&self) -> &Context {
        self.runtime_ctx.as_ref()
    }
    pub fn set_context(&mut self, ctx: Context) {
        self.runtime_ctx = Arc::new(ctx);
    }

    pub fn exec(
        &self,
        _guard: TaskGuard,
        node_id: String,
        node: Arc<RwLock<dyn AnyNode>>,
        input: Input,
        // 向调度器发送执行结果
        task_sender: mpsc::Sender<TaskEvent>,
    ) {
        // 节点执行所需的上下文
        let ctx = self.runtime_ctx.clone();
        let semaphore = self.semaphore.clone();
        tokio::spawn(async move {
            let _keep_alive = _guard; // Force capture
            let _permit = match semaphore {
                Some(sem) => match sem.acquire_owned().await {
                    Ok(permit) => Some(permit),
                    Err(e) => {
                        send_task_event(
                            &task_sender,
                            TaskEvent::Error(node_id, format!("Semaphore acquire failed: {}", e)),
                        )
                        .await;
                        return;
                    }
                },
                None => None,
            };
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
                        send_task_event(
                            &task_sender,
                            TaskEvent::Completed(node_id, out.into_value()),
                        )
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
