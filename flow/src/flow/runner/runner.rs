use super::{
    exec_context::{ExecutionContext, NodeState},
    executor::Executor,
    utils::send_flow_event,
};
use crate::flow::{
    event::{FlowEvent, TaskEvent},
    graph::{Context, Graph, NodeId, TriggerStrategy},
    io::{Input, Output},
};

use serde_json::Value;

use std::{
    collections::{HashMap, VecDeque},
    sync::Arc,
};
use tokio::sync::{mpsc, watch};
use tracing::{debug, error, info, trace};

type TaskPayload = (Vec<NodeId>, Input);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunnerState {
    Initial,
    Running,
    Paused,
    Terminated,
}

pub enum RunnerCommand {
    Pause,
    Resume,
    Stop,
}

#[derive(Clone)]
pub struct RunnerHandle {
    cmd_tx: mpsc::Sender<RunnerCommand>,
    state_rx: watch::Receiver<RunnerState>,
}

impl RunnerHandle {
    pub async fn pause(&self) {
        let _ = self.cmd_tx.send(RunnerCommand::Pause).await;
    }

    pub async fn resume(&self) {
        let _ = self.cmd_tx.send(RunnerCommand::Resume).await;
    }

    pub async fn stop(&self) {
        let _ = self.cmd_tx.send(RunnerCommand::Stop).await;
    }

    pub fn get_state(&self) -> RunnerState {
        *self.state_rx.borrow()
    }
}

pub struct Runner {
    graph: Graph,
    ctx: Arc<Context>,
    task_queue: VecDeque<TaskPayload>,
    // 执行上下文，提供调度决策信息
    exec_ctx: ExecutionContext,
    executor: Executor,
    // 内部运行状态
    state_tx: watch::Sender<RunnerState>,
    // 外部事件发送通道
    event_sender: Option<mpsc::Sender<FlowEvent>>,
    // 外部命令接收通道
    cmd_rx: mpsc::Receiver<RunnerCommand>,
}

impl Runner {
    pub fn new(graph: Graph, initial_context: Option<ExecutionContext>) -> (Self, RunnerHandle) {
        let (cmd_tx, cmd_rx) = mpsc::channel(32);
        let (state_tx, state_rx) = watch::channel(RunnerState::Initial);

        let handle = RunnerHandle {
            cmd_tx,
            state_rx: state_rx.clone(),
        };

        (
            Self {
                graph,
                ctx: Arc::new(Context::new()),
                task_queue: VecDeque::new(),
                exec_ctx: initial_context.unwrap_or_else(ExecutionContext::new),
                executor: Executor::new(),
                event_sender: None,
                cmd_rx,
                state_tx,
            },
            handle,
        )
    }

    pub fn set_event_sender(&mut self, sender: mpsc::Sender<FlowEvent>) {
        self.event_sender = Some(sender);
    }

    pub fn set_context(&mut self, ctx: Context) {
        self.ctx = Arc::new(ctx);
    }

    pub fn set_max_concurrency(&mut self, max: usize) {
        self.executor.set_max_concurrency(max);
    }

    pub fn clear_max_concurrency(&mut self) {
        self.executor.clear_max_concurrency();
    }

    pub fn get_execution_context(&self) -> &ExecutionContext {
        &self.exec_ctx
    }

    pub fn set_start_node(&mut self, node_id: &str, input: Value) {
        let mut inputs = HashMap::new();
        inputs.insert("external_start".to_owned(), input);

        self.task_queue
            .push_back((vec![node_id.to_owned()], Input::new(inputs)));
    }

    pub fn set_start_node_with_inputs(&mut self, node_id: &str, inputs: Input) {
        self.task_queue
            .push_back((vec![node_id.to_owned()], inputs));
    }

    fn update_runner_state(&self, state: RunnerState) -> Result<(), String> {
        self.state_tx
            .send(state)
            .map_err(|_| "Failed to update runner state".to_owned())
    }

    pub async fn run(&mut self) -> Result<(), String> {
        info!(nodes = ?self.graph.get_node_ids(), "Runner started");

        if let Some(sender) = self.event_sender.as_ref() {
            self.ctx.set_any("__flow_event_sender", sender.clone());
        }

        if self.task_queue.is_empty() {
            info!("No start node set, runner finished");
            return Ok(());
        }
        let init_state = *self.state_tx.borrow();
        if init_state == RunnerState::Initial {
            self.update_runner_state(RunnerState::Running)?;
        }
        self.exec_ctx.reset_task_count(); // 重置任务计数器
        let (task_sender, mut rx) = mpsc::channel::<TaskEvent>(128);
        // run 只返回最错的报错信息, 如果有
        let mut first_error = None;

        // 初始启动：将 task_queue 中的初始任务直接启动
        while let Some((node_ids, inputs)) = self.task_queue.pop_front() {
            for node_id in node_ids {
                self.start_node(node_id, inputs.clone(), task_sender.clone())
                    .await?;
            }
        }

        loop {
            let current_state = *self.state_tx.borrow();

            // 整个 loop 就干三件事
            // 1. 接收外部 command，外部通过 handle 传入 cmd 可能改变内部状态
            // 2. 根据内部状态判断是否需要处理任务事件
            // 3. 检查终止条件，没有任务为止
            tokio::select! {
                // 1. 处理外部命令
                Some(cmd) = self.cmd_rx.recv() => {
                    match cmd {
                        RunnerCommand::Pause => {
                            if current_state != RunnerState::Terminated {
                                info!("Runner paused");
                                self.update_runner_state(RunnerState::Paused)?;
                                send_flow_event(&self.event_sender, FlowEvent::FlowPaused).await;
                            }
                        }
                        RunnerCommand::Resume => {
                            if current_state == RunnerState::Paused {
                                info!("Runner resumed");
                                self.update_runner_state(RunnerState::Running)?;
                                send_flow_event(&self.event_sender, FlowEvent::FlowResumed).await;
                            }
                        }
                        RunnerCommand::Stop => {
                            info!("Runner terminated by command");
                            self.update_runner_state(RunnerState::Terminated)?;
                            send_flow_event(&self.event_sender, FlowEvent::FlowStopped).await;
                            break;
                        }
                    }
                }

                // 2. 处理任务
                maybe_event = rx.recv(), if current_state == RunnerState::Running => {
                    match maybe_event {
                        Some(first_event) => {
                            // 批量获取当前队列中的所有事件
                            let mut events = vec![first_event];
                            while let Ok(event) = rx.try_recv() {
                                events.push(event);
                            }

                            for task_event in events {
                                self.handle_task_event(task_event, task_sender.clone(), &mut first_error).await?;
                            }
                        }
                        None => {
                            // 没有事件了，继续等待
                        }
                    }
                }
            }
            // 没有活跃任务，且事件队列为空，就意味着工作流执行完毕
            if self.exec_ctx.get_task_count() == 0
                && rx.is_empty()
                && current_state == RunnerState::Running
            {
                info!("Runner finished: All tasks completed.",);
                self.update_runner_state(RunnerState::Terminated)?;
                send_flow_event(&self.event_sender, FlowEvent::FlowFinished).await;
                break;
            }
        }

        if let Some(e) = first_error {
            Err(e)
        } else {
            Ok(())
        }
    }

    async fn handle_task_event(
        &mut self,
        event: TaskEvent,
        task_sender: mpsc::Sender<TaskEvent>,
        first_error: &mut Option<String>,
    ) -> Result<(), String> {
        match event {
            // 任务返回流式结果则订阅并通知事件流开始
            TaskEvent::Stream(node_id, subscribe_fn) => {
                info!(node_id = %node_id, "Received Stream");
                let sub = subscribe_fn(
                    self.exec_ctx.get_task_tracker_guard(),
                    task_sender,
                    node_id.clone(),
                    self.ctx.clone(),
                );
                self.exec_ctx.set_stream_subscription(node_id.clone(), sub);
                send_flow_event(&self.event_sender, FlowEvent::NodeStreamStarted(node_id)).await;
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
                send_flow_event(
                    &self.event_sender,
                    FlowEvent::NodeStreamNextMessage(node_id.clone(), output.clone()),
                )
                .await;
                // 触发下游节点调度
                self.trigger_downstream(node_id, output, task_sender)
                    .await?
            }
            TaskEvent::Completed(node_id, output) => {
                info!(node_id = %node_id, "Node tasks completed");
                let _ = self.exec_ctx.remove_stream_subscription(&node_id);
                let mut out_value = None;
                if let Some(out) = output {
                    self.exec_ctx.set_output(node_id.clone(), out.clone());
                    send_flow_event(
                        &self.event_sender,
                        FlowEvent::NodeOutMessage(node_id.clone(), out.clone()),
                    )
                    .await;
                    out_value = Some(out);
                }
                self.exec_ctx
                    .set_state(node_id.clone(), NodeState::Completed);
                send_flow_event(
                    &self.event_sender,
                    FlowEvent::NodeCompleted(node_id.clone()),
                )
                .await;
                // 如果有输出值，就触发下游调度，流式输出完成时没有值，就不触发下游调度了
                if let Some(out) = out_value {
                    self.trigger_downstream(node_id, out, task_sender).await?
                }
            }
            TaskEvent::Error(node_id, e) => {
                error!(node_id = %node_id, error = %e, "Node execution failed");
                let _ = self.exec_ctx.remove_stream_subscription(&node_id);
                if first_error.is_none() {
                    *first_error = Some(e.clone());
                }
                self.exec_ctx.set_state(node_id.clone(), NodeState::Failed);
                send_flow_event(&self.event_sender, FlowEvent::NodeError(node_id, e)).await;
            }
        }
        Ok(())
    }

    // 下游节点触发取决于两个条件：
    // 1. 下游条件边返回 true
    // 2. 满足下游节点的调度策略
    async fn trigger_downstream(
        &mut self,
        from_node_id: String,
        output: Value,
        tx: mpsc::Sender<TaskEvent>,
    ) -> Result<(), String> {
        // 找到所有满足条件的下游节点
        let next_nodes = self.find_next_nodes(&from_node_id, &output)?;
        trace!(node_id = %from_node_id, next_nodes = ?next_nodes, "Triggering dependent nodes");
        for next_node_id in next_nodes {
            self.check_and_schedule(next_node_id, from_node_id.clone(), tx.clone())
                .await?;
        }
        Ok(())
    }

    fn find_next_nodes(&self, from_node_id: &str, output: &Value) -> Result<Vec<String>, String> {
        let mut next_nodes = vec![];
        let output = Output::new(Some(output.clone()));
        if let Some(edges) = self.graph.edges.get(from_node_id) {
            trace!(from = %from_node_id, count = edges.len(), "Found outgoing edges");
            for edge in edges.iter() {
                let passes = edge.check_condition(&self.ctx, &output);
                trace!(
                    from = %edge.from(),
                    to = %edge.to(),
                    passes = passes,
                    "Edge condition check"
                );
                if passes {
                    next_nodes.push(edge.to().to_owned())
                }
            }
        } else {
            trace!(from = %from_node_id, "No outgoing edges found");
        }
        Ok(next_nodes)
    }

    async fn check_and_schedule(
        &mut self,
        node_id: String,
        from_node_id: String,
        task_sender: mpsc::Sender<TaskEvent>,
    ) -> Result<(), String> {
        match self.graph.get_trigger_strategy(&node_id) {
            // 任意上游节点就绪即触发
            TriggerStrategy::AnyUpstreamAvailable => {
                trace!(node_id = %node_id, "Any dependency met, scheduling");
                let mut inputs_map = HashMap::new();
                if let Some(output) = self.exec_ctx.get_output(&from_node_id) {
                    inputs_map.insert(from_node_id, output);
                }
                let inputs = Input::new(inputs_map);
                self.start_node(node_id.to_string(), inputs, task_sender)
                    .await
            }
            // 所有上游节点就绪才触发
            TriggerStrategy::AllUpstreamReady => {
                trace!(node_id = %node_id, "All dependency met, scheduling");
                let mut inputs_map = HashMap::new();
                let parents = self.graph.get_parents(&node_id);
                for parent_id in parents {
                    let state = self.exec_ctx.get_state(&parent_id);
                    match state {
                        Some(NodeState::Completed) => {
                            // Collect input from parent
                            if let Some(output) = self.exec_ctx.get_output(&parent_id) {
                                inputs_map.insert(parent_id.clone(), output);
                            }
                        }
                        Some(NodeState::Skipped) => {
                            // Dependency met but no data
                        }
                        _ => {
                            // Parent not ready (Pending, Running, Failed, or None)
                            return Ok(());
                        }
                    }
                }
                let inputs = Input::new(inputs_map);
                self.start_node(node_id.to_string(), inputs, task_sender)
                    .await
            }
        }
    }

    async fn start_node(
        &mut self,
        node_id: NodeId,
        inputs: Input,
        task_sender: mpsc::Sender<TaskEvent>,
    ) -> Result<(), String> {
        debug!(
            node_id = %node_id,
            total_active = self.exec_ctx.get_task_count(),
            "Starting node task"
        );
        // 准备节点执行数据
        // 节点实例
        let node_arc = self
            .graph
            .nodes
            .get(&node_id)
            .ok_or_else(|| format!("Runner start_node: Node '{}' not found", &node_id))?
            .clone();

        // 节点运行上下文
        let ctx = self.ctx.clone();
        // 节点任务计数守卫
        let guard = self.exec_ctx.get_task_tracker_guard();

        send_flow_event(&self.event_sender, FlowEvent::NodeStarted(node_id.clone())).await;
        // 发送节点输入事件，通知外部，输入的数据不一定能序列化
        send_flow_event(
            &self.event_sender,
            FlowEvent::NodeInMessage(node_id.clone(), inputs.clone()),
        )
        .await;
        // 让执行器干活
        self.executor
            .exec(guard, node_id, node_arc, inputs, ctx, task_sender);
        Ok(())
    }
}
