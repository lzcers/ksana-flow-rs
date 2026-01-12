use crate::flow::sendable_any::SendableAny;

use super::graph::{AnyNode, Context, Graph, NodeId, TaskEvent};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc, mpsc::Sender};
use tracing::{error, info};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum FlowEvent {
    NodeStarted(String),
    NodeCompleted(String),
    NodeError(String, String),
    Finished,
}

type TaskPayload = (Vec<NodeId>, Box<dyn SendableAny>);

pub struct Runner {
    graph: Graph,
    ctx: Arc<Context>,
    task_queue: VecDeque<TaskPayload>,
    active_tasks: HashMap<NodeId, usize>,
    total_active_tasks: usize,
    event_sender: Option<Sender<FlowEvent>>,
}

impl Runner {
    pub fn new(graph: Graph) -> Self {
        Self {
            graph,
            ctx: Arc::new(Context::new()),
            task_queue: VecDeque::new(),
            active_tasks: HashMap::new(),
            total_active_tasks: 0,
            event_sender: None,
        }
    }

    pub fn set_event_sender(mut self, sender: Sender<FlowEvent>) -> Self {
        self.event_sender = Some(sender);
        self
    }

    pub fn set_start_node(mut self, node_id: &str, input: &dyn SendableAny) -> Self {
        self.task_queue
            .push_back((vec![node_id.to_owned()], input.clone_box()));
        self
    }

    pub async fn run(&mut self) -> Result<(), String> {
        info!("Available nodes: {:?}", self.graph.get_node_ids());
        let (task_sender, mut rx) = mpsc::channel::<TaskEvent>(128);
        // 使用计数器跟踪每个节点的活跃任务数
        self.active_tasks.clear();
        self.total_active_tasks = 0;

        // 初始启动：将 task_queue 中的初始任务直接启动
        while let Some((node_ids, input)) = self.task_queue.pop_front() {
            for node_id in node_ids {
                self.start_node(node_id, input.clone(), task_sender.clone())?;
            }
        }

        let mut first_error = None;

        while let Some(first_event) = rx.recv().await {
            // 批量获取当前队列中的所有事件，以便进行优先级排序
            let mut events = vec![first_event];
            while let Ok(event) = rx.try_recv() {
                events.push(event);
            }

            // 优先级排序：Completed/Error > Stream > Next
            // 这样可以确保下游节点的完成事件优先于上游节点的新数据产生事件被处理
            // 从而实现更好的任务交织，防止上游流数据阻塞整个流水线
            events.sort_by_key(|event| match event {
                TaskEvent::Completed(..) | TaskEvent::Error(..) => 0,
                TaskEvent::Stream(..) => 1,
                TaskEvent::Next(..) => 2,
            });

            for task_event in events {
                match task_event {
                    TaskEvent::Stream(node_id, subscribe_fn) => {
                        info!("Detected reactive stream from node: {}", &node_id);
                        // 响应式流作为一个长运行任务，已经在 active_tasks 中计数
                        // 它会由 subscribe_fn 发送 Completed 或 Error 来结束
                        let _sub = subscribe_fn(task_sender.clone(), node_id, self.ctx.clone());
                    }
                    TaskEvent::Next(node_id, output) => {
                        info!(
                            "Received Next event from node: {}. Output type: {}",
                            &node_id,
                            output.as_ref().type_name()
                        );
                        // 寻找并启动下游节点
                        self.trigger_downstream(&node_id, output, task_sender.clone())?;
                    }
                    TaskEvent::Completed(node_id, output) => {
                        // 1. 处理节点产出的数据（如果是普通节点）
                        if let Some(out) = output {
                            info!(
                                "Received Completed with data from node: {}. Output type: {}",
                                &node_id,
                                out.as_ref().type_name()
                            );
                            self.trigger_downstream(&node_id, out, task_sender.clone())?;
                        }
                        // 2. 更新任务计数器
                        self.update_active_tasks(&node_id);
                    }
                    TaskEvent::Error(node_id, e) => {
                        error!("Node '{}' error: {}", node_id, e);
                        if first_error.is_none() {
                            first_error = Some(e);
                        }
                        self.update_active_tasks(&node_id);
                    }
                }
            }

            if self.total_active_tasks == 0 {
                info!("All tasks finished, exiting runner loop.");
                break;
            }
        }

        Self::send_flow_event(&self.event_sender, FlowEvent::Finished).await;

        if let Some(e) = first_error {
            return Err(e);
        }
        Ok(())
    }

    fn trigger_downstream(
        &mut self,
        from_node_id: &str,
        output: Box<dyn SendableAny>,
        tx: Sender<TaskEvent>,
    ) -> Result<(), String> {
        let next_nodes = self.find_next_nodes(from_node_id, &output)?;
        info!(
            "Found {} next nodes for {}: {:?}",
            &next_nodes.len(),
            from_node_id,
            &next_nodes
        );
        for next_node_id in next_nodes {
            self.start_node(next_node_id, output.clone(), tx.clone())?;
        }
        Ok(())
    }

    fn start_node(
        &mut self,
        node_id: NodeId,
        input: Box<dyn SendableAny>,
        task_sender: Sender<TaskEvent>,
    ) -> Result<(), String> {
        let node_arc = self
            .graph
            .nodes
            .get(&node_id)
            .ok_or_else(|| format!("Runner start_node: Node '{}' not found", &node_id))?
            .clone();

        *self.active_tasks.entry(node_id.clone()).or_insert(0) += 1;
        self.total_active_tasks += 1;

        info!(
            "Task started: {}. Total active tasks: {}",
            &node_id, self.total_active_tasks
        );

        let event_sender = self.event_sender.clone();
        Self::worker(
            node_id,
            node_arc,
            self.ctx.clone(),
            input,
            task_sender,
            event_sender,
        );
        Ok(())
    }

    async fn send_flow_event(sender: &Option<Sender<FlowEvent>>, event: FlowEvent) {
        if let Some(sender) = sender {
            let _ = sender.send(event).await;
        }
    }
    async fn send_task_event(sender: &Sender<TaskEvent>, event: TaskEvent) {
        let _ = sender.send(event).await;
    }

    //创建一个异步任务运行节点
    fn worker(
        node_id: String,
        node: Arc<RwLock<dyn AnyNode>>,
        ctx: Arc<Context>,
        input: Box<dyn SendableAny>,
        task_sender: Sender<TaskEvent>,
        event_sender: Option<Sender<FlowEvent>>,
    ) {
        tokio::spawn(async move {
            Self::send_flow_event(&event_sender, FlowEvent::NodeStarted(node_id.clone())).await;

            let mut node = node.write().await;
            let output = node.run(&ctx, input).await;

            info!(
                "Finished running node: <{}> in task: {:?}",
                &node_id,
                tokio::task::id(),
            );

            Self::send_flow_event(&event_sender, FlowEvent::NodeCompleted(node_id.clone())).await;

            match output {
                Ok(out) => {
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
            self.total_active_tasks -= 1;
            info!(
                "Task {} active count: {}. Total active: {}",
                &node_id, *count, self.total_active_tasks
            );
            if *count == 0 {
                info!(
                    "Task completed: {}. Total active: {}",
                    &node_id, self.total_active_tasks
                );
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
                let passes = edge.check_condition(&self.ctx, output.as_any());
                info!(
                    "Edge <{}> -> <{}> condition: [{}]",
                    edge.from(),
                    edge.to(),
                    passes
                );
                if passes {
                    next_nodes.push(edge.to().to_owned())
                }
            }
        }
        Ok(next_nodes)
    }
}
