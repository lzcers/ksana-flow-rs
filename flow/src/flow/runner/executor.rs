//! 节点任务执行层。
//!
//! Executor 负责并发限制、panic 隔离、取消普通节点任务以及把结果转换成
//! `TaskEvent`。工作流状态和下游调度仍由 Runner 串行处理。

use super::{event::TaskEvent, task_guard::TaskGuard};
use crate::{
    Context, ControllerHandle, RunnerId,
    flow::{
        AnyNode,
        graph::{Input, Output},
    },
    scope_current_node, scope_runner,
};
use futures::FutureExt;
use std::{
    panic::AssertUnwindSafe,
    sync::{Arc, Mutex},
};
use tokio::{
    sync::{
        RwLock, Semaphore,
        mpsc::{self, error::TryRecvError},
    },
    task::JoinSet,
};
use tokio_util::sync::CancellationToken;
use tracing::{debug, info};

/// Runner 私有的异步节点执行器。
///
/// `JoinSet` 只跟踪 `exec` 创建的节点任务；`ReactiveStream` 产生的流任务由
/// `ExecutionContext` 单独管理。
pub struct Executor {
    semaphore: Option<Arc<Semaphore>>,
    runtime_ctx: Arc<Context>,
    task_sender: mpsc::Sender<TaskEvent>,
    task_receiver: mpsc::Receiver<TaskEvent>,
    cancel: CancellationToken,
    tasks: Mutex<JoinSet<()>>,
}

impl Executor {
    pub fn new() -> Self {
        let (task_sender, task_receiver) = mpsc::channel::<TaskEvent>(128);
        Self {
            runtime_ctx: Arc::new(Context::new()),
            semaphore: None,
            task_sender,
            task_receiver,
            cancel: CancellationToken::new(),
            tasks: Mutex::new(JoinSet::new()),
        }
    }

    /// 替换后续任务使用的并发信号量。
    ///
    /// 已经捕获旧信号量的运行中或等待中任务不会迁移到新上限。
    pub fn set_max_concurrency(&mut self, max: usize) {
        let max = max.max(1);
        self.semaphore = Some(Arc::new(Semaphore::new(max)));
    }

    pub fn clear_max_concurrency(&mut self) {
        self.semaphore = None;
    }

    pub fn get_runtime_context(&self) -> Arc<Context> {
        self.runtime_ctx.clone()
    }
    pub fn get_runtime_context_ref(&self) -> &Context {
        self.runtime_ctx.as_ref()
    }

    pub fn set_runtime_context(&mut self, ctx: Context) {
        self.runtime_ctx = Arc::new(ctx);
    }

    pub fn get_task_sender(&self) -> mpsc::Sender<TaskEvent> {
        self.task_sender.clone()
    }
    pub async fn get_task_event(&mut self) -> Option<TaskEvent> {
        self.task_receiver.recv().await
    }
    pub fn try_get_task_event(&mut self) -> Result<TaskEvent, TryRecvError> {
        self.task_receiver.try_recv()
    }
    pub fn event_is_empty(&self) -> bool {
        self.task_receiver.is_empty()
    }

    /// 取消 CancellationToken 并终止当前 JoinSet 中的所有节点任务。
    pub fn cancel(&self) {
        self.cancel.cancel();
        if let Ok(mut tasks) = self.tasks.lock() {
            tasks.abort_all();
            while tasks.try_join_next().is_some() {}
        }
    }

    pub fn reap_finished(&self) {
        if let Ok(mut tasks) = self.tasks.lock() {
            while tasks.try_join_next().is_some() {}
        }
    }

    /// 提交一次节点执行并立即返回。
    ///
    /// `TaskGuard` 被移动到异步任务中，确保结果事件发送完成前任务计数不会归零。
    /// 节点写锁会串行化同一节点实例上的多次执行。
    pub fn exec(
        &self,
        runner_id: RunnerId,
        controller: ControllerHandle,
        _guard: TaskGuard,
        node_id: String,
        node: Arc<RwLock<dyn AnyNode>>,
        input: Input,
    ) {
        // 节点执行所需的上下文
        let ctx = self.runtime_ctx.clone();
        let semaphore = self.semaphore.clone();
        let task_sender = self.get_task_sender();
        let cancel = self.cancel.clone();
        let node_id_for_scope = node_id.clone();

        let fut = scope_runner(
            controller,
            runner_id,
            scope_current_node(node_id_for_scope, async move {
                let _keep_alive = _guard; // Force capture
                let node_id_for_cancel = node_id.clone();
                tokio::select! {
                    _ = cancel.cancelled() => {
                        Self::send_task_event(&task_sender, TaskEvent::Error(node_id_for_cancel, "Cancelled".to_string())).await;
                        return;
                    }
                    _ = async {
                        let _permit = match semaphore {
                            Some(sem) => match sem.acquire_owned().await {
                                Ok(permit) => Some(permit),
                                Err(e) => {
                                    Self::send_task_event(
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
                                            Self::send_task_event(
                                                &task_sender,
                                                TaskEvent::Stream(node_id, stream.start),
                                            )
                                            .await;
                                        }
                                        None => {
                                            Self::send_task_event(
                                                &task_sender,
                                                TaskEvent::Error(node_id, "Missing stream".to_string()),
                                            )
                                            .await;
                                        }
                                    }
                                } else {
                                    Self::send_task_event(
                                        &task_sender,
                                        TaskEvent::Completed(node_id, out.into_value()),
                                    )
                                    .await;
                                }
                            }
                            Err(e) => {
                                Self::send_task_event(&task_sender, TaskEvent::Error(node_id, e)).await;
                            }
                        }
                    } => {}
                }
            }),
        );

        if let Ok(mut tasks) = self.tasks.lock() {
            tasks.spawn(fut);
        } else {
            tokio::spawn(fut);
        }
    }
    async fn send_task_event(task_sender: &mpsc::Sender<TaskEvent>, event: TaskEvent) {
        let _ = task_sender.send(event).await;
    }
}

impl Drop for Executor {
    fn drop(&mut self) {
        self.cancel.cancel();
        if let Ok(tasks) = self.tasks.get_mut() {
            tasks.abort_all();
            while tasks.try_join_next().is_some() {}
        }
    }
}
