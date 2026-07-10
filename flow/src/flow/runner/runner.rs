use super::{
    event::{FlowEvent, FlowEventEnvelope, TaskEvent},
    exec_context::{ExecutionContext, NodeState},
    executor::Executor,
    logger,
    scheduler::{Scheduler, StartSpec},
};
use crate::{
    Context, ControllerHandle, RunnerId,
    flow::graph::{Graph, Input, NodeId},
    scope_current_node, scope_runner,
};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{broadcast, mpsc, watch};
use tracing::{Instrument, debug, trace, warn};

/// Runner 主循环的生命周期状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunnerState {
    Initial,
    Running,
    Paused,
    Terminated,
}

/// 发往 Controller 执行树的控制命令。
///
/// Controller 使用 broadcast 分发，因此命令不是单个 Runner 私有的。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunnerCommand {
    Pause,
    Resume,
    Stop,
    SetMaxConcurrency(usize),
    ClearMaxConcurrency,
}

/// 对 Controller 执行树发送命令，并观察创建该句柄时对应 Runner 的状态。
#[derive(Clone)]
pub struct RunnerHandle {
    cmd_tx: broadcast::Sender<RunnerCommand>,
    state_rx: watch::Receiver<RunnerState>,
}

impl RunnerHandle {
    /// 暂停 Runner 对新 `TaskEvent` 的处理。
    ///
    /// 已经提交给 Executor 的节点和流生产者仍会继续运行，结果会留在内部通道中
    /// 等待恢复处理；这不是对底层异步任务的抢占式暂停。
    pub async fn pause(&self) {
        let _ = self.cmd_tx.send(RunnerCommand::Pause);
    }

    pub async fn resume(&self) {
        let _ = self.cmd_tx.send(RunnerCommand::Resume);
    }

    /// 停止同一 Controller 下的 Runner。Runner 会取消 Executor 管理的节点任务。
    pub async fn stop(&self) {
        let _ = self.cmd_tx.send(RunnerCommand::Stop);
    }

    pub fn get_state(&self) -> RunnerState {
        *self.state_rx.borrow()
    }
}

/// 一次 Graph 执行的主协调器。
///
/// Runner 自身不执行节点逻辑：Scheduler 判断就绪，Executor 运行节点，
/// ExecutionContext 保存状态；Runner 负责串行处理命令和任务事件以推进状态机。
pub struct Runner {
    // 1. 工作流的scheduler，负责任务的调度和执行
    // 驱动整个工作流执行
    scheduler: Scheduler,
    // 2. 执行上下文，存储节点状态于执行结果，提供调度决策信息
    exec_ctx: ExecutionContext,
    // 3. 任务执行器，负责执行任务
    executor: Executor,
    runner_id: RunnerId,
    // 内部运行状态
    state_tx: watch::Sender<RunnerState>,
    // Runner 开始时间，用于计算总耗时
    start_time: Option<Instant>,
    // 外部命令接收通道
    controller: ControllerHandle,
    cmd_rx: broadcast::Receiver<RunnerCommand>,
    flow_event_tx: mpsc::Sender<FlowEventEnvelope>,
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

        let mut exec_ctx = initial_context.unwrap_or_else(ExecutionContext::new);
        // 设置 runner_id 用于日志记录
        exec_ctx.set_runner_id(runner_id);
        let flow_event_tx = controller.get_flow_event_sender();
        let executor = Executor::new();
        (
            Self {
                scheduler: Scheduler::new(graph),
                exec_ctx,
                executor,
                runner_id,
                controller,
                state_tx,
                cmd_rx: handle.cmd_tx.subscribe(),
                flow_event_tx,
                start_time: None,
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

    pub fn set_start_node(&mut self, node_id: &str, inputs: Input) {
        self.scheduler.enqueue_start(node_id, inputs);
    }

    pub(crate) fn apply_max_concurrency(&mut self, max: usize) {
        self.executor.set_max_concurrency(max);
    }

    /// 执行当前 start queue 中的工作流。
    ///
    /// 完成条件是任务计数归零且内部事件队列为空。该方法会等待外部 FlowEvent
    /// 进入 Controller 的有界通道，因此事件消费者也参与整体背压。
    pub async fn run(&mut self) -> Result<(), String> {
        // 记录 Runner 启动
        self.start_time = Some(Instant::now());
        let node_count = self.scheduler.get_node_ids().len();
        logger::log_runner_started(self.runner_id, &self.scheduler.get_node_ids(), node_count);

        // 创建 runner span 用于追踪整个执行过程
        let runner_span = logger::create_runner_span(self.runner_id, node_count);

        async move {
            // 1.准备阶段
            // 各种状态初始化
            // 节点实例化等

            if self.scheduler.is_start_queue_empty() {
                logger::log_runner_terminated(self.runner_id,"No start node set");
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
            let mut terminated_by_stop = false;

            // 2. 启动阶段
            // 从 scheduler 中弹出初始启动节点
            let starts = self.scheduler.pop_initial_starts();
            self.start_by_specs(starts).await?;
            self.send_flow_event(FlowEvent::FlowStarted).await;

            // 3. 执行阶段
            // 整个 loop 就干三件事
            // 1. 接收外部 command，外部通过 handle 传入 cmd 可能改变内部状态
            // 2. 根据内部状态判断是否需要处理任务事件
            // 3. 检查终止条件，没有任务为止
            loop {
                // 本轮 select 使用状态快照；命令造成的状态变化从下一轮开始控制分支。
                let current_state = *self.state_tx.borrow();
                tokio::select! {
                    // 1. 处理外部命令
                    cmd_res = self.cmd_rx.recv() => {
                        match cmd_res {
                            Ok(cmd) => match cmd {
                                RunnerCommand::Pause => {
                                    if current_state != RunnerState::Terminated {
                                        self.update_runner_state(RunnerState::Paused)?;
                                        self.send_flow_event( FlowEvent::FlowPaused).await;
                                    }
                                }
                                RunnerCommand::Resume => {
                                    if current_state == RunnerState::Paused {
                                        self.update_runner_state(RunnerState::Running)?;
                                        self.send_flow_event( FlowEvent::FlowResumed).await;
                                    }
                                }
                                RunnerCommand::Stop => {
                                    logger::log_runner_terminated(self.runner_id, "Stop command received");
                                    self.executor.cancel();
                                    self.update_runner_state(RunnerState::Terminated)?;
                                    self.send_flow_event(FlowEvent::FlowStopped).await;
                                    terminated_by_stop = true;
                                    break;
                                }
                                RunnerCommand::SetMaxConcurrency(max) => {
                                    self.executor.set_max_concurrency(max);
                                }
                                RunnerCommand::ClearMaxConcurrency => {
                                    self.executor.clear_max_concurrency();
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
                // TaskGuard 覆盖普通节点任务和流订阅任务；只有两类任务均释放守卫，
                // 且它们发出的 TaskEvent 已处理完，Runner 才能自然结束。
                if self.exec_ctx.get_task_count() == 0
                    && self.executor.event_is_empty()
                    && current_state == RunnerState::Running
                {
                    self.update_runner_state(RunnerState::Terminated)?;
                    self.send_flow_event(FlowEvent::FlowFinished).await;
                    break;
                }
            }

            // 存在未完成的节点
            // 如果有未完成节点，且未被停止，则记录警告日志
            let node_ids = self.scheduler.get_node_ids();
            let mut incomplete_nodes = Vec::new();
            for node_id in &node_ids {
                let state = self.exec_ctx.get_state(node_id);
                let is_done =
                    matches!(state, Some(NodeState::Completed | NodeState::Failed | NodeState::Skipped));
                if !is_done {
                    incomplete_nodes.push((node_id.clone(), state));
                }
            }

            if !incomplete_nodes.is_empty() {
                if !terminated_by_stop && first_error.is_none() {
                    let incomplete_node_ids: Vec<_> =
                        incomplete_nodes.iter().map(|(id, _)| id.clone()).collect();
                    warn!(
                        runner_id = self.runner_id,
                        incomplete_nodes = ?incomplete_node_ids,
                        "Runner terminated with incomplete nodes (graph disconnected, missing edges, or AllUpstreamReady blocked)"
                    );
                }
                for (node_id, state) in &incomplete_nodes {
                    let parents = self.scheduler.get_parents(node_id);
                    let parent_states: Vec<_> = parents
                        .iter()
                        .map(|p| (p.clone(), self.exec_ctx.get_state(p)))
                        .collect();
                    debug!(
                        node_id,
                        node_state = ?state,
                        parent_states = ?parent_states,
                        "Node did not complete before runner termination"
                    );
                }
            }

            // 记录 Runner 完成
            if let Some(start_time) = self.start_time {
                logger::log_runner_completed(self.runner_id, start_time.elapsed());
            }

            if let Some(e) = first_error {
                Err(e)
            } else {
                Ok(())
            }
        }
        .instrument(runner_span)
        .await
    }

    fn update_runner_state(&self, new_state: RunnerState) -> Result<(), String> {
        let old_state = *self.state_tx.borrow();

        // 只在状态变化时记录日志
        if old_state != new_state {
            logger::log_runner_state_change(self.runner_id, old_state, new_state);
        }

        self.state_tx
            .send(new_state)
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
                logger::log_stream_started(&node_id, self.runner_id);
                let guard = self.exec_ctx.get_task_tracker_guard();
                let task_sender = self.executor.get_task_sender();
                let runtime_ctx = self.executor.get_runtime_context();
                let controller = self.controller.clone();
                let node_id_for_sub = node_id.clone();
                let node_id_for_scope = node_id.clone();
                let runner_id = self.runner_id;
                // 订阅闭包只消费一次，并返回取消句柄；ExecutionContext 持有该句柄，
                // 直到流结束或出错时由相应 TaskEvent 分支移除。
                let sub = scope_runner(
                    controller,
                    runner_id,
                    scope_current_node(node_id_for_scope, async move {
                        subscribe_fn(guard, task_sender, node_id_for_sub, runtime_ctx)
                    }),
                )
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
                // 获取节点执行耗时
                let elapsed = self.exec_ctx.get_node_elapsed(&node_id);
                let has_output = output.is_some();

                // 记录节点执行完成
                if let Some(duration) = elapsed {
                    logger::log_node_execution_completed(
                        &node_id,
                        self.runner_id,
                        duration,
                        has_output,
                    );
                }

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
                logger::log_node_execution_error(&node_id, self.runner_id, &e);
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
        // 记录节点执行开始
        let active_count = self.exec_ctx.get_task_count();
        logger::log_node_execution_start(&node_id, self.runner_id, active_count);

        // 准备节点执行数据
        // 节点实例
        let node_arc = self
            .scheduler
            .get_node(&node_id)
            .ok_or_else(|| format!("Runner start_node: Node '{}' not found", &node_id))?;

        // 守卫必须在 exec 前创建并移动进任务，防止任务尚未登记时 Runner 误判完成。
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
        // 每次发送都根据注册表计算完整子图路径。发送是有背压的；慢事件消费者会
        // 延缓状态机推进，但不会改变同一 Runner 内事件的先后顺序。
        let (runner_kind, parent_runner_id, parent_node_id, subgraph_path) =
            self.controller.describe_runner(self.runner_id);
        let envelope = FlowEventEnvelope {
            runner_id: self.runner_id,
            runner_kind,
            parent_runner_id,
            parent_node_id,
            subgraph_path,
            event,
        };
        let _ = self.flow_event_tx.send(envelope).await;
    }
}
