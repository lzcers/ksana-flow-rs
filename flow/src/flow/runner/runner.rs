use crate::flow::{
    event::{FlowEvent, TaskEvent},
    graph::{AnyNode, Context, Graph, NodeId},
    runner::task_guard::{TaskGuard, TaskTracker},
    sendable_any::{SendableAny, try_downcast_to_value},
};
use std::{
    collections::{HashMap, VecDeque},
    sync::Arc,
};
use tokio::sync::{RwLock, mpsc, watch};
use tracing::{debug, error, info, trace};

type TaskPayload = (Vec<NodeId>, Box<dyn SendableAny>);

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
                event_sender: None,
                cmd_rx,
                state_tx,
            },
            handle,
        )
    }

    pub fn set_event_sender(mut self, sender: mpsc::Sender<FlowEvent>) -> Self {
        self.event_sender = Some(sender);
        self
    }

    pub fn set_start_node(mut self, node_id: &str, input: &dyn SendableAny) -> Self {
        self.task_queue
            .push_back((vec![node_id.to_owned()], input.clone_box()));
        self
    }

    pub async fn run(&mut self) -> Result<(), String> {
        info!(nodes = ?self.graph.get_node_ids(), "Runner started");

        if *self.state_tx.borrow() == RunnerState::Initial {
            let _ = self.state_tx.send(RunnerState::Running);
        }

        let (task_sender, mut rx) = mpsc::channel::<TaskEvent>(128);
        // 使用计数器跟踪每个节点的活跃任务数
        self.active_tasks.clear();

        // 初始启动：将 task_queue 中的初始任务直接启动
        while let Some((node_ids, input)) = self.task_queue.pop_front() {
            for node_id in node_ids {
                self.start_node(node_id, input.clone(), task_sender.clone())?;
            }
        }

        if self.tracker.count() == 0 {
            info!("Runner finished: No tasks started");
            Self::send_flow_event(&self.event_sender, FlowEvent::Finished).await;
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
                                let _ = self.state_tx.send(RunnerState::Paused);
                                info!("Runner paused");
                                Self::send_flow_event(&self.event_sender, FlowEvent::FlowPaused).await;
                            }
                        }
                        RunnerCommand::Resume => {
                            if current_state == RunnerState::Paused {
                                let _ = self.state_tx.send(RunnerState::Running);
                                info!("Runner resumed");
                                Self::send_flow_event(&self.event_sender, FlowEvent::FlowResumed).await;
                            }
                        }
                        RunnerCommand::Stop => {
                            let _ = self.state_tx.send(RunnerState::Terminated);
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
                            // 批量获取当前队列中的所有事件，以便进行优先级排序
                            let mut events = vec![first_event];
                            while let Ok(event) = rx.try_recv() {
                                events.push(event);
                            }

                            // 优先级排序：Completed/Error > Stream > Next
                            events.sort_by_key(|event| match event {
                                TaskEvent::Completed(..) | TaskEvent::Error(..) => 0,
                                TaskEvent::Stream(..) => 1,
                                TaskEvent::Next(..) => 2,
                            });

                            for task_event in events {
                                self.handle_task_event(task_event, task_sender.clone(), &mut first_error)?;
                            }
                        }
                        None => {
                            break;
                        }
                    }
                }
                // 3. 检查终止条件，没有任务为止
                _ = self.tracker.await_notify() => {
                    if self.tracker.count() == 0 && *self.state_tx.borrow() == RunnerState::Running {
                        info!("Runner finished: All tasks completed");
                        break;
                    }
                }
            }
        }

        Self::send_flow_event(&self.event_sender, FlowEvent::Finished).await;

        if let Some(e) = first_error {
            return Err(e);
        }
        Ok(())
    }

    fn handle_task_event(
        &mut self,
        event: TaskEvent,
        task_sender: mpsc::Sender<TaskEvent>,
        first_error: &mut Option<String>,
    ) -> Result<(), String> {
        match event {
            TaskEvent::Stream(node_id, subscribe_fn) => {
                info!(node_id = %node_id, "Received Stream");
                let _sub = subscribe_fn(task_sender, node_id, self.ctx.clone());
            }
            TaskEvent::Next(node_id, output) => {
                info!(
                    node_id = %node_id,
                    output_type = %output.as_ref().type_name(),
                    "Received Next event"
                );
                self.trigger_downstream(&node_id, output, task_sender)?;
            }
            TaskEvent::Completed(node_id, output) => {
                if let Some(out) = output {
                    self.trigger_downstream(&node_id, out, task_sender)?;
                }
                self.update_active_tasks(&node_id);
            }
            TaskEvent::Error(node_id, e) => {
                error!(node_id = %node_id, error = %e, "Node execution failed");
                if first_error.is_none() {
                    *first_error = Some(e);
                }
                self.update_active_tasks(&node_id);
            }
        }
        Ok(())
    }

    fn trigger_downstream(
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
            self.start_node(next_node_id, output.clone(), tx.clone())?;
        }
        Ok(())
    }

    fn start_node(
        &mut self,
        node_id: NodeId,
        input: Box<dyn SendableAny>,
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
        // 节点事件发送器
        let event_sender = self.event_sender.clone();
        // 节点运行上下文
        let ctx = self.ctx.clone();
        // 节点任务计数守卫
        let guard = TaskGuard::new(self.tracker.clone());
        *self.active_tasks.entry(node_id.clone()).or_insert(0) += 1;
        Self::worker(
            node_id,
            node_arc,
            ctx,
            input,
            task_sender,
            event_sender,
            guard,
        );
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

    //创建一个异步任务运行节点
    fn worker(
        node_id: String,
        node: Arc<RwLock<dyn AnyNode>>,
        ctx: Arc<Context>,
        input: Box<dyn SendableAny>,
        task_sender: mpsc::Sender<TaskEvent>,
        event_sender: Option<mpsc::Sender<FlowEvent>>,
        _guard: TaskGuard,
    ) {
        tokio::spawn(async move {
            let _keep_alive = _guard; // Force capture
            Self::send_flow_event(&event_sender, FlowEvent::NodeStarted(node_id.clone())).await;

            let mut node = node.write().await;

            // 尝试将输入转换为 Value 并发送到 Web 端
            if let Some(val) = try_downcast_to_value(input.as_ref()) {
                Self::send_flow_event(
                    &event_sender,
                    FlowEvent::NodeInMessage(node_id.clone(), val),
                )
                .await;
            }

            let output: Result<Box<dyn SendableAny>, String> = node.run(&ctx, input).await;

            debug!(node_id = %node_id, "Node logic executed");

            Self::send_flow_event(&event_sender, FlowEvent::NodeCompleted(node_id.clone())).await;

            match output {
                Ok(out) => {
                    // 如果节点输出是个响应式流，需要订阅它
                    if out.as_ref().is_stream() {
                        match out.into_stream_subscriber() {
                            Ok(subscribe_fn) => {
                                Self::send_task_event(
                                    &task_sender,
                                    TaskEvent::Stream(node_id, subscribe_fn),
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
                        // 尝试将输出转换为 Value 并发送到 Web 端
                        if let Some(val) = try_downcast_to_value(out.as_ref()) {
                            Self::send_flow_event(
                                &event_sender,
                                FlowEvent::NodeOutMessage(node_id.clone(), val),
                            )
                            .await;
                        }
                        // 非流式节点，发送带数据的 Completed 事件
                        Self::send_task_event(
                            &task_sender,
                            TaskEvent::Completed(node_id, Some(out)),
                        )
                        .await;
                    }
                }
                Err(e) => {
                    Self::send_flow_event(
                        &event_sender,
                        FlowEvent::NodeError(node_id.clone(), e.clone()),
                    )
                    .await;
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
                info!(node_id = %node_id, "Node tasks completed");
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
            for edge in edges.iter() {
                // Pass the inner value as &dyn Any
                let passes = edge.check_condition(&self.ctx, (*output).as_any());
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
        }
        Ok(next_nodes)
    }
}
