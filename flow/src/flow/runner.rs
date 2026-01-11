use super::graph::{AnyNode, CloneAny, Context, Graph, NodeId, TaskEvent};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc, mpsc::Sender};
use tracing::{error, info};

type TaskPayload = (Vec<NodeId>, Box<dyn CloneAny>);

pub struct Runner {
    graph: Graph,
    ctx: Arc<Context>,
    task_queue: VecDeque<TaskPayload>,
    active_tasks: HashMap<NodeId, usize>,
    total_active_tasks: usize,
}

impl Runner {
    pub fn new(graph: Graph) -> Self {
        Self {
            graph,
            ctx: Arc::new(Context::new()),
            task_queue: VecDeque::new(),
            active_tasks: HashMap::new(),
            total_active_tasks: 0,
        }
    }

    pub fn set_start_node(mut self, node_id: &str, input: &dyn CloneAny) -> Self {
        self.task_queue
            .push_back((vec![node_id.to_owned()], input.clone_box()));
        self
    }

    pub async fn run(&mut self) -> Result<(), String> {
        info!("Available nodes: {:?}", self.graph.get_node_ids());
        let (tx, mut rx) = mpsc::channel::<TaskEvent>(1000);
        // 使用计数器跟踪每个节点的活跃任务数
        self.active_tasks.clear();
        self.total_active_tasks = 0;

        // 初始启动：将 task_queue 中的初始任务直接启动
        while let Some((node_ids, input)) = self.task_queue.pop_front() {
            for node_id in node_ids {
                self.start_node(node_id, input.clone_box(), tx.clone())?;
            }
        }

        while let Some(task_event) = rx.recv().await {
            match task_event {
                TaskEvent::Stream(node_id, subscribe_fn) => {
                    info!("Detected reactive stream from node: {}", &node_id);
                    // 响应式流作为一个长运行任务，已经在 active_tasks 中计数
                    // 它会由 subscribe_fn 发送 Completed 或 Error 来结束
                    let _sub = subscribe_fn(tx.clone(), node_id, self.ctx.clone());
                }
                TaskEvent::Next(node_id, output) => {
                    info!(
                        "Received Next event from node: {}. Output type: {}",
                        &node_id,
                        output.as_ref().type_name()
                    );
                    // 寻找并启动下游节点
                    self.trigger_downstream(&node_id, output, tx.clone())?;
                }
                TaskEvent::Completed(node_id, output) => {
                    // 1. 处理节点产出的数据（如果是普通节点）
                    if let Some(out) = output {
                        info!(
                            "Received Completed with data from node: {}. Output type: {}",
                            &node_id,
                            out.as_ref().type_name()
                        );
                        self.trigger_downstream(&node_id, out, tx.clone())?;
                    }
                    // 2. 更新任务计数器
                    self.update_active_tasks(&node_id);
                }
                TaskEvent::Error(node_id, e) => {
                    error!("Node '{}' error: {}", node_id, e);
                    self.update_active_tasks(&node_id);
                }
            }

            if self.total_active_tasks == 0 {
                info!("All tasks finished, exiting runner loop.");
                break;
            }
        }

        Ok(())
    }

    fn trigger_downstream(
        &mut self,
        from_node_id: &str,
        output: Box<dyn CloneAny>,
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
        input: Box<dyn CloneAny>,
        tx: Sender<TaskEvent>,
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

        Self::worker(node_id, node_arc, self.ctx.clone(), input, tx);
        Ok(())
    }

    //创建一个异步任务运行节点
    fn worker(
        node_id: String,
        node: Arc<RwLock<dyn AnyNode>>,
        ctx: Arc<Context>,
        input: Box<dyn CloneAny>,
        tx: Sender<TaskEvent>,
    ) {
        tokio::spawn(async move {
            let mut node = node.write().await;
            let output = node.run(&ctx, input).await;

            info!(
                "Finished running node: <{}> in task: {:?}",
                &node_id,
                tokio::task::id(),
            );

            match output {
                Ok(out) => {
                    if out.as_ref().is_stream() {
                        match out.into_stream_subscriber() {
                            Ok(subscribe_fn) => {
                                let _ = tx.send(TaskEvent::Stream(node_id, subscribe_fn)).await;
                            }
                            Err(e) => {
                                let _ = tx
                                    .send(TaskEvent::Error(
                                        node_id,
                                        format!(
                                            "Failed to get stream subscriber: {}",
                                            e.type_name()
                                        ),
                                    ))
                                    .await;
                            }
                        }
                    } else {
                        // 非流式节点，发送带数据的 Completed 事件
                        let _ = tx.send(TaskEvent::Completed(node_id, Some(out))).await;
                    }
                }
                Err(e) => {
                    let _ = tx.send(TaskEvent::Error(node_id, e)).await;
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
        output: &Box<dyn CloneAny>,
    ) -> Result<Vec<String>, String> {
        let mut next_nodes = vec![];
        if let Some(edges) = self.graph.edges.get(from_node_id) {
            for edge in edges.iter() {
                let passes = edge.check_condition(&self.ctx, output.as_ref());
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
