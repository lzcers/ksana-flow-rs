use super::context::{ExecutionContext, NodeState};
use crate::{
    flatten_sendable_any,
    flow::{
        event::{FlowEvent, TaskEvent},
        graph::{AnyNode, Context, Graph, NodeId, NodeInputs},
        runner::task_guard::{TaskGuard, TaskTracker},
        sendable_any::{SendableAny, try_downcast_to_value},
    },
};
use futures::FutureExt;
use std::panic::AssertUnwindSafe;
use std::{
    collections::{HashMap, VecDeque},
    sync::Arc,
};
use tokio::sync::{RwLock, mpsc, watch};
use tracing::{debug, error, info, trace};

type TaskPayload = (Vec<NodeId>, NodeInputs);

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
    active_tasks: HashMap<NodeId, usize>,
    tracker: Arc<TaskTracker>,
    execution_ctx: ExecutionContext,
    // 内部运行状态
    state_tx: watch::Sender<RunnerState>,
    // 外部事件发送通道
    event_sender: Option<mpsc::Sender<FlowEvent>>,
    // 外部命令接收通道
    cmd_rx: mpsc::Receiver<RunnerCommand>,
}

impl Runner {
    pub fn new(graph: Graph) -> (Self, RunnerHandle) {
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
                active_tasks: HashMap::new(),
                tracker: Arc::new(TaskTracker::new()),
                execution_ctx: ExecutionContext::new(),
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

    pub fn set_start_node(&mut self, node_id: &str, input: &dyn SendableAny) {
        let mut inputs = HashMap::new();
        let boxed_input = input.clone_box();
        let boxed_input = flatten_sendable_any(boxed_input);
        inputs.insert("external_start".to_owned(), boxed_input);

        self.task_queue
            .push_back((vec![node_id.to_owned()], NodeInputs::new(inputs)));
    }

    pub async fn run(&mut self) -> Result<(), String> {
        info!(nodes = ?self.graph.get_node_ids(), "Runner started");
        let init_state = *self.state_tx.borrow();
        if init_state == RunnerState::Initial {
            self.update_runner_state(RunnerState::Running)?;
        }

        self.active_tasks.clear();
        let (task_sender, mut rx) = mpsc::channel::<TaskEvent>(128);
        // 使用计数器跟踪每个节点的活跃任务数

        // 初始启动：将 task_queue 中的初始任务直接启动
        while let Some((node_ids, inputs)) = self.task_queue.pop_front() {
            for node_id in node_ids {
                let mut inputs_map = HashMap::new();
                for (k, v) in &inputs.inputs {
                    inputs_map.insert(k.clone(), v.clone_box());
                }

                self.start_node(node_id, NodeInputs::new(inputs_map), task_sender.clone())
                    .await?;
            }
        }

        if self.tracker.count() == 0 {
            info!("Runner finished: No tasks started");
            Self::send_flow_event(&self.event_sender, FlowEvent::FlowFinished).await;
            return Ok(());
        }

        let mut first_error = None;

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
                                self.update_runner_state(RunnerState::Paused)?;
                                info!("Runner paused");
                                Self::send_flow_event(&self.event_sender, FlowEvent::FlowPaused).await;
                            }
                        }
                        RunnerCommand::Resume => {
                            if current_state == RunnerState::Paused {
                                self.update_runner_state(RunnerState::Running)?;
                                info!("Runner resumed");
                                Self::send_flow_event(&self.event_sender, FlowEvent::FlowResumed).await;
                            }
                        }
                        RunnerCommand::Stop => {
                            self.update_runner_state(RunnerState::Terminated)?;
                            info!("Runner terminated by command");
                            Self::send_flow_event(&self.event_sender, FlowEvent::FlowStopped).await;
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
                            break;
                        }
                    }
                }
            }

            // 3. 检查终止条件
            if self.tracker.count() == 0
                && self.active_tasks.is_empty()
                && *self.state_tx.borrow() == RunnerState::Running
            {
                break;
            }
        }
        info!("Runner finished: All tasks completed.",);
        self.update_runner_state(RunnerState::Terminated)?;
        Self::send_flow_event(&self.event_sender, FlowEvent::FlowFinished).await;
        if let Some(e) = first_error {
            return Err(e);
        }
        Ok(())
    }

    async fn handle_task_event(
        &mut self,
        event: TaskEvent,
        task_sender: mpsc::Sender<TaskEvent>,
        first_error: &mut Option<String>,
    ) -> Result<(), String> {
        match event {
            TaskEvent::Stream(node_id, subscribe_fn) => {
                info!(node_id = %node_id, "Received Stream");
                let _sub = subscribe_fn(task_sender, node_id.clone(), self.ctx.clone());
                Self::send_flow_event(&self.event_sender, FlowEvent::NodeStreamStarted(node_id))
                    .await;
            }
            TaskEvent::Next(node_id, output) => {
                trace!(
                    node_id = %node_id,
                    output_type = %output.as_ref().type_name(),
                    "Received Next event"
                );
                if let Some(val) = try_downcast_to_value(output.as_ref()) {
                    Self::send_flow_event(
                        &self.event_sender,
                        FlowEvent::NodeStreamNextMessage(node_id.clone(), val),
                    )
                    .await;
                }
                // self.trigger_downstream(&node_id, output, task_sender)
                //     .await?;
            }
            TaskEvent::Completed(node_id, output) => {
                trace!(node_id = %node_id, has_output = output.is_some(), "Received Completed event");
                if let Some(out) = output {
                    // Check if it's already a Box<dyn SendableAny> wrapper, and if so, unwrap it
                    let out = flatten_sendable_any(out);

                    // 尝试将输出转换为 Value 并发送到 Web 端
                    if let Some(val) = try_downcast_to_value(out.as_ref()) {
                        Self::send_flow_event(
                            &self.event_sender,
                            FlowEvent::NodeOutMessage(node_id.clone(), val),
                        )
                        .await;
                    }
                    Self::send_flow_event(
                        &self.event_sender,
                        FlowEvent::NodeCompleted(node_id.clone()),
                    )
                    .await;
                    self.execution_ctx
                        .set_state(node_id.clone(), NodeState::Completed);
                    self.execution_ctx.set_output(node_id.clone(), out.clone());
                    // Trigger downstream with dependency check
                    let next_nodes = self.find_next_nodes(&node_id, &out)?;

                    trace!(node_id = %node_id, next_nodes = ?next_nodes, "Triggering dependent nodes");

                    for next_node_id in next_nodes {
                        self.check_and_schedule(&next_node_id, task_sender.clone())
                            .await?;
                    }
                } else {
                    // 对于流式输出的 Completed，只发送 Completed 事件
                    Self::send_flow_event(
                        &self.event_sender,
                        FlowEvent::NodeCompleted(node_id.clone()),
                    )
                    .await;
                    self.execution_ctx
                        .set_state(node_id.clone(), NodeState::Completed);
                }
                info!(node_id = %node_id, "Node tasks completed");
                self.update_active_tasks(&node_id);
            }
            TaskEvent::Error(node_id, e) => {
                error!(node_id = %node_id, error = %e, "Node execution failed");
                if first_error.is_none() {
                    *first_error = Some(e.clone());
                }

                self.execution_ctx
                    .set_state(node_id.clone(), NodeState::Failed);

                Self::send_flow_event(&self.event_sender, FlowEvent::NodeError(node_id.clone(), e))
                    .await;
                self.update_active_tasks(&node_id);
            }
        }
        Ok(())
    }

    async fn check_and_schedule(
        &mut self,
        node_id: &str,
        task_sender: mpsc::Sender<TaskEvent>,
    ) -> Result<(), String> {
        let parents = self.graph.get_parents(node_id);
        trace!(node_id = %node_id, parents = ?parents, "Checking dependencies");

        let mut inputs_map = HashMap::new();

        for parent_id in &parents {
            let state = self.execution_ctx.get_state(parent_id);
            trace!(parent = %parent_id, state = ?state, "Parent state");
            match state {
                Some(NodeState::Completed) => {
                    // Collect input from parent
                    if let Some(output) = self.execution_ctx.get_output(parent_id) {
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

        // All parents ready
        trace!(node_id = %node_id, "All dependencies met, scheduling");
        let inputs = NodeInputs::new(inputs_map);

        self.start_node(node_id.to_string(), inputs, task_sender)
            .await?;
        Ok(())
    }

    async fn trigger_downstream(
        &mut self,
        from_node_id: &str,
        output: Box<dyn SendableAny>,
        tx: mpsc::Sender<TaskEvent>,
    ) -> Result<(), String> {
        let next_nodes = self.find_next_nodes(from_node_id, &output)?;
        if !next_nodes.is_empty() {
            debug!(
                from = %from_node_id,
                targets = ?next_nodes,
                "Triggering downstream nodes"
            );
        }
        for next_node_id in next_nodes {
            let mut inputs = HashMap::new();
            inputs.insert(from_node_id.to_owned(), output.clone_box());

            self.start_node(next_node_id, NodeInputs::new(inputs), tx.clone())
                .await?;
        }
        Ok(())
    }

    async fn start_node(
        &mut self,
        node_id: NodeId,
        inputs: NodeInputs,
        task_sender: mpsc::Sender<TaskEvent>,
    ) -> Result<(), String> {
        // 节点实例
        let node_arc = self
            .graph
            .nodes
            .get(&node_id)
            .ok_or_else(|| format!("Runner start_node: Node '{}' not found", &node_id))?
            .clone();
        debug!(
            node_id = %node_id,
            total_active = self.tracker.count(),
            "Starting node task"
        );
        // 节点运行上下文
        let ctx = self.ctx.clone();
        // 节点任务计数守卫
        let guard = TaskGuard::new(self.tracker.clone());
        *self.active_tasks.entry(node_id.clone()).or_insert(0) += 1;

        Self::send_flow_event(&self.event_sender, FlowEvent::NodeStarted(node_id.clone())).await;

        let mut inputs_json = serde_json::Map::new();
        for (key, val) in &inputs.inputs {
            if let Some(v) = try_downcast_to_value(val.as_ref()) {
                inputs_json.insert(key.clone(), v);
            }
        }

        if !inputs_json.is_empty() {
            Self::send_flow_event(
                &self.event_sender,
                FlowEvent::NodeInMessage(node_id.clone(), serde_json::Value::Object(inputs_json)),
            )
            .await;
        }

        Self::worker(node_id, node_arc, ctx, inputs, task_sender, guard);
        Ok(())
    }

    async fn send_flow_event(sender: &Option<mpsc::Sender<FlowEvent>>, event: FlowEvent) {
        if let Some(sender) = sender {
            let _ = sender.send(event).await;
        }
    }

    async fn send_task_event(sender: &mpsc::Sender<TaskEvent>, event: TaskEvent) {
        let _ = sender.send(event).await;
    }

    fn update_runner_state(&self, state: RunnerState) -> Result<(), String> {
        self.state_tx
            .send(state)
            .map_err(|_| "Failed to update runner state".to_owned())
    }
    //创建一个异步任务运行节点
    fn worker(
        node_id: String,
        node: Arc<RwLock<dyn AnyNode>>,
        ctx: Arc<Context>,
        inputs: NodeInputs,
        task_sender: mpsc::Sender<TaskEvent>,
        _guard: TaskGuard,
    ) {
        tokio::spawn(async move {
            let _keep_alive = _guard; // Force capture
            let mut node = node.write().await;

            info!(node_id = %node_id, "Node tasks start");
            let result = AssertUnwindSafe(node.run(&ctx, inputs))
                .catch_unwind()
                .await;

            let output: Result<Box<dyn SendableAny>, String> = match result {
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
                    // 如果节点输出是个响应式流，需要订阅它
                    if out.as_ref().is_stream() {
                        match out.into_stream_subscriber() {
                            Ok(subscribe_fn) => {
                                Self::send_task_event(
                                    &task_sender,
                                    TaskEvent::Stream(node_id.clone(), subscribe_fn),
                                )
                                .await;
                            }
                            Err(e) => {
                                Self::send_task_event(
                                    &task_sender,
                                    TaskEvent::Error(
                                        node_id,
                                        format!(
                                            "Failed to get stream subscriber: {}",
                                            e.type_name()
                                        ),
                                    ),
                                )
                                .await;
                            }
                        }
                    } else {
                        Self::send_task_event(
                            &task_sender,
                            TaskEvent::Completed(node_id, Some(out)),
                        )
                        .await;
                    }
                }
                Err(e) => {
                    Self::send_task_event(&task_sender, TaskEvent::Error(node_id, e)).await;
                }
            }
        });
    }

    fn update_active_tasks(&mut self, node_id: &str) {
        if let Some(count) = self.active_tasks.get_mut(node_id) {
            *count -= 1;
            trace!(
                node_id = %node_id,
                count = count,
                total = self.tracker.count(),
                "Task count updated"
            );
            if *count == 0 {
                self.active_tasks.remove(node_id);
            }
        }
    }

    fn find_next_nodes(
        &self,
        from_node_id: &str,
        output: &Box<dyn SendableAny>,
    ) -> Result<Vec<String>, String> {
        let mut next_nodes = vec![];
        if let Some(edges) = self.graph.edges.get(from_node_id) {
            trace!(from = %from_node_id, count = edges.len(), "Found outgoing edges");
            for edge in edges.iter() {
                // Pass the inner value as &dyn Any
                let passes = edge.check_condition(&self.ctx, (*output).as_any());
                trace!(
                    from = %edge.from(),
                    to = %edge.to(),
                    passes = passes,
                    output_type = %output.as_ref().type_name(),
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
}
