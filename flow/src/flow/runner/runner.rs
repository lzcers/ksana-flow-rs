use super::{
    event::{FlowEvent, TaskEvent},
    exec_context::{ExecutionContext, NodeState},
    executor::Executor,
    scheduler::{Scheduler, StartSpec},
};
use crate::{
    Context, ControllerHandle, RunnerId,
    flow::graph::{Graph, Input, NodeId},
    scope_runner,
};
use std::sync::Arc;
use tokio::sync::{broadcast, watch};
use tracing::{debug, error, info, trace};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunnerState {
    Initial,
    Running,
    Paused,
    Terminated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunnerCommand {
    Pause,
    Resume,
    Stop,
}

#[derive(Clone)]
pub struct RunnerHandle {
    cmd_tx: broadcast::Sender<RunnerCommand>,
    state_rx: watch::Receiver<RunnerState>,
}

impl RunnerHandle {
    pub async fn pause(&self) {
        let _ = self.cmd_tx.send(RunnerCommand::Pause);
    }

    pub async fn resume(&self) {
        let _ = self.cmd_tx.send(RunnerCommand::Resume);
    }

    pub async fn stop(&self) {
        let _ = self.cmd_tx.send(RunnerCommand::Stop);
    }

    pub fn get_state(&self) -> RunnerState {
        *self.state_rx.borrow()
    }
}

pub struct Runner {
    // 1. 工作流的scheduler，负责任务的调度和执行
    // 驱动整个工作流执行
    scheduler: Scheduler,
    // 2. 执行上下文，存储节点状态于执行结果，提供调度决策信息
    exec_ctx: ExecutionContext,
    // 3. 任务执行器，负责执行任务
    executor: Executor,
    runner_id: RunnerId,
    controller: ControllerHandle,
    // 内部运行状态
    state_tx: watch::Sender<RunnerState>,
    // 外部命令接收通道
    cmd_rx: broadcast::Receiver<RunnerCommand>,
}

impl Runner {
    pub(crate) fn new(
        graph: Arc<Graph>,
        // 用于恢复执行的上下文
        initial_context: Option<ExecutionContext>,
        controller: ControllerHandle,
        runner_id: RunnerId,
    ) -> (Self, RunnerHandle) {
        let (state_tx, state_rx) = watch::channel(RunnerState::Initial);

        let handle = RunnerHandle {
            cmd_tx: controller.cmd_tx(),
            state_rx: state_rx.clone(),
        };

        let executor = Executor::new();
        (
            Self {
                scheduler: Scheduler::new(graph),
                exec_ctx: initial_context.unwrap_or_else(ExecutionContext::new),
                executor,
                runner_id,
                controller,
                state_tx,
                cmd_rx: handle.cmd_tx.subscribe(),
            },
            handle,
        )
    }

    pub fn set_runtime_context(&mut self, ctx: Context) {
        self.executor.set_runtime_context(ctx);
    }

    pub fn get_execution_context(&self) -> &ExecutionContext {
        &self.exec_ctx
    }

    pub fn set_max_concurrency(&mut self, max: usize) {
        self.executor.set_max_concurrency(max);
    }

    pub fn clear_max_concurrency(&mut self) {
        self.executor.clear_max_concurrency();
    }

    pub fn set_start_node(&mut self, node_id: &str, inputs: Input) {
        self.scheduler.enqueue_start(node_id, inputs);
    }

    pub async fn run(&mut self) -> Result<(), String> {
        info!(nodes = ?self.scheduler.get_node_ids(), "Runner started");

        // 1.准备阶段
        // 各种状态初始化
        // 节点实例化等

        if self.scheduler.is_start_queue_empty() {
            info!("No start node set, runner finished");
            return Ok(());
        }
        let init_state = *self.state_tx.borrow();
        if init_state == RunnerState::Initial {
            self.update_runner_state(RunnerState::Running)?;
        }
        self.scheduler.materialize_nodes().await?;
        self.exec_ctx.reset_task_count(); // 重置任务计数器

        // run 只返回最错的报错信息, 如果有
        let mut first_error = None;

        // 2. 启动阶段
        // 从 scheduler 中弹出初始启动节点
        let starts = self.scheduler.pop_initial_starts();
        self.start_by_specs(starts).await?;

        // 3. 执行阶段
        // 整个 loop 就干三件事
        // 1. 接收外部 command，外部通过 handle 传入 cmd 可能改变内部状态
        // 2. 根据内部状态判断是否需要处理任务事件
        // 3. 检查终止条件，没有任务为止
        loop {
            let current_state = *self.state_tx.borrow();
            tokio::select! {
                // 1. 处理外部命令
                cmd_res = self.cmd_rx.recv() => {
                    match cmd_res {
                        Ok(cmd) => match cmd {
                            RunnerCommand::Pause => {
                                if current_state != RunnerState::Terminated {
                                    info!("Runner paused");
                                    self.update_runner_state(RunnerState::Paused)?;
                                    self.send_flow_event( FlowEvent::FlowPaused).await;
                                }
                            }
                            RunnerCommand::Resume => {
                                if current_state == RunnerState::Paused {
                                    info!("Runner resumed");
                                    self.update_runner_state(RunnerState::Running)?;
                                    self.send_flow_event( FlowEvent::FlowResumed).await;
                                }
                            }
                            RunnerCommand::Stop => {
                                info!("Runner terminated by command");
                                self.executor.cancel();
                                self.update_runner_state(RunnerState::Terminated)?;
                                self.send_flow_event(FlowEvent::FlowStopped).await;
                                break;
                            }
                        },
                        Err(broadcast::error::RecvError::Lagged(_)) => {
                            continue;
                        }
                        Err(broadcast::error::RecvError::Closed) => {
                            break;
                        }
                    };
                }

                // 2. 处理任务
                maybe_event = self.executor.get_task_event(), if current_state == RunnerState::Running => {
                    match maybe_event {
                        Some(first_event) => {
                            // 批量获取当前队列中的所有事件
                            let mut events = vec![first_event];
                            while let Ok(event) = self.executor.try_get_task_event() {
                                events.push(event);
                            }

                            for task_event in events {
                                self.handle_task_event(task_event, &mut first_error).await?;
                            }
                        }
                        None => {
                            // 没有事件了，继续等待
                        }
                    }
                }
            }
            self.executor.reap_finished();
            // 没有活跃任务，且事件队列为空，就意味着工作流执行完毕
            if self.exec_ctx.get_task_count() == 0
                && self.executor.event_is_empty()
                && current_state == RunnerState::Running
            {
                info!("Runner finished: All tasks completed.",);
                self.update_runner_state(RunnerState::Terminated)?;
                self.send_flow_event(FlowEvent::FlowFinished).await;
                break;
            }
        }

        if let Some(e) = first_error {
            Err(e)
        } else {
            Ok(())
        }
    }

    fn update_runner_state(&self, state: RunnerState) -> Result<(), String> {
        self.state_tx
            .send(state)
            .map_err(|_| "Failed to update runner state".to_owned())
    }

    async fn handle_task_event(
        &mut self,
        event: TaskEvent,
        first_error: &mut Option<String>,
    ) -> Result<(), String> {
        match event {
            // 任务返回流式结果则订阅并通知事件流开始
            TaskEvent::Stream(node_id, subscribe_fn) => {
                info!(node_id = %node_id, "Received Stream");
                let guard = self.exec_ctx.get_task_tracker_guard();
                let task_sender = self.executor.get_task_sender();
                let runtime_ctx = self.executor.get_runtime_context();
                let controller = self.controller.clone();
                let node_id_for_sub = node_id.clone();
                let sub = scope_runner(controller, self.runner_id, async move {
                    subscribe_fn(guard, task_sender, node_id_for_sub, runtime_ctx)
                })
                .await;
                self.exec_ctx.set_stream_subscription(node_id.clone(), sub);
                self.send_flow_event(FlowEvent::NodeStreamStarted(node_id))
                    .await;
            }
            // 任务的流式结果
            TaskEvent::Next(node_id, output) => {
                trace!(
                    node_id = %node_id,
                    output = ?output,
                    "Received Next event"
                );

                // 流式输出存储到 exec_ctx 中
                self.exec_ctx.set_output(node_id.clone(), output.clone());
                self.send_flow_event(FlowEvent::NodeStreamNextMessage(
                    node_id.clone(),
                    output.clone(),
                ))
                .await;
                let starts = self.scheduler.schedule_from_output(
                    &node_id,
                    &output,
                    &self.exec_ctx,
                    self.executor.get_runtime_context_ref(),
                )?;
                self.start_by_specs(starts).await?;
            }
            TaskEvent::Completed(node_id, output) => {
                info!(node_id = %node_id, "Node tasks completed");
                let _ = self.exec_ctx.remove_stream_subscription(&node_id);
                let mut out_value = None;
                if let Some(out) = output {
                    self.exec_ctx.set_output(node_id.clone(), out.clone());
                    self.send_flow_event(FlowEvent::NodeOutMessage(node_id.clone(), out.clone()))
                        .await;
                    out_value = Some(out);
                }
                self.exec_ctx
                    .set_state(node_id.clone(), NodeState::Completed);
                self.send_flow_event(FlowEvent::NodeCompleted(node_id.clone()))
                    .await;
                // 如果有输出值，就触发下游调度，流式输出完成时没有值，就不触发下游调度了
                if let Some(out) = out_value {
                    let starts = self.scheduler.schedule_from_output(
                        &node_id,
                        &out,
                        &self.exec_ctx,
                        self.executor.get_runtime_context_ref(),
                    )?;
                    self.start_by_specs(starts).await?;
                }
            }
            TaskEvent::Error(node_id, e) => {
                error!(node_id = %node_id, error = %e, "Node execution failed");
                let _ = self.exec_ctx.remove_stream_subscription(&node_id);
                if first_error.is_none() {
                    *first_error = Some(e.clone());
                }
                self.exec_ctx.set_state(node_id.clone(), NodeState::Failed);
                self.send_flow_event(FlowEvent::NodeError(node_id, e)).await;
            }
        }
        Ok(())
    }

    async fn start_by_specs(&mut self, starts: Vec<StartSpec>) -> Result<(), String> {
        for spec in starts {
            self.start_node(spec.node_id, spec.inputs).await?;
        }
        Ok(())
    }

    async fn start_node(&mut self, node_id: NodeId, inputs: Input) -> Result<(), String> {
        debug!(
            node_id = %node_id,
            total_active = self.exec_ctx.get_task_count(),
            "Starting node task"
        );
        // 准备节点执行数据
        // 节点实例
        let node_arc = self
            .scheduler
            .get_node(&node_id)
            .ok_or_else(|| format!("Runner start_node: Node '{}' not found", &node_id))?;

        // 节点任务计数守卫
        let guard = self.exec_ctx.get_task_tracker_guard();

        self.exec_ctx.set_state(node_id.clone(), NodeState::Running);

        self.send_flow_event(FlowEvent::NodeStarted(node_id.clone()))
            .await;
        // 发送节点输入事件，通知外部，输入的数据不一定能序列化
        self.send_flow_event(FlowEvent::NodeInMessage(node_id.clone(), inputs.clone()))
            .await;
        // 让执行器干活
        self.executor.exec(
            self.runner_id,
            self.controller.clone(),
            guard,
            node_id,
            node_arc,
            inputs,
        );
        Ok(())
    }

    async fn send_flow_event(&self, event: FlowEvent) {
        self.controller.send_event(event).await;
    }
}
