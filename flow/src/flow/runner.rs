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
}

impl Runner {
    pub fn new(graph: Graph) -> Self {
        Self {
            graph,
            ctx: Arc::new(Context::new()),
            task_queue: VecDeque::new(),
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
        let mut active_tasks: HashMap<NodeId, usize> = HashMap::new();
        let mut total_active_tasks: usize = 0;

        // 初始启动：将 task_queue 中的初始任务直接启动
        // 接下来就靠节点的消息驱动，每个节点的输出会触发下一个节点的运行
        while let Some((node_ids, input)) = self.task_queue.pop_front() {
            for node_id in node_ids {
                let node_arc = self
                    .graph
                    .nodes
                    .get(&node_id)
                    .ok_or_else(|| format!("Runner run: Node '{}' not found", &node_id))?
                    .clone();
                let ctx_arc = self.ctx.clone();
                let input_clone = input.clone();
                let tx_clone = tx.clone();
                // 对应节点的活跃任务计数器 +1
                *active_tasks.entry(node_id.clone()).or_insert(0) += 1;
                total_active_tasks += 1;
                info!(
                    "Task started: {}. Active tasks count: {}",
                    &node_id, total_active_tasks
                );

                Self::worker(node_id, node_arc, ctx_arc, input_clone, tx_clone);
            }
        }

        while let Some(task_event) = rx.recv().await {
            // 接收节点运行的结果
            match task_event {
                TaskEvent::Next(node_id, output) => {
                    info!(
                        "Received Next event from node: {}. Output type: {}",
                        &node_id,
                        output.as_ref().type_name()
                    );
                    // 尝试将输出转换为流订阅器
                    match output.into_stream_subscriber() {
                        Ok(subscribe_fn) => {
                            info!("Detected reactive stream from node: {}", &node_id);
                            // 响应式流作为一个长运行任务，已经在 active_tasks 中计数
                            // 它会由 subscribe_fn 发送 Completed 或 Error 来结束
                            let _sub = subscribe_fn(tx.clone(), node_id, self.ctx.clone());
                        }
                        Err(original_output) => {
                            // 寻找并启动下游节点
                            let next_nodes = self.find_next_nodes(&node_id, &original_output)?;
                            info!(
                                "Found {} next nodes for {}: {:?}",
                                next_nodes.len(),
                                &node_id,
                                next_nodes
                            );
                            for next_node_id in next_nodes {
                                let node_arc = self
                                    .graph
                                    .nodes
                                    .get(&next_node_id)
                                    .ok_or_else(|| {
                                        format!("Runner run: Node '{}' not found", &next_node_id)
                                    })?
                                    .clone();
                                let ctx_arc = self.ctx.clone();
                                let input_clone = original_output.clone();
                                let tx_clone = tx.clone();

                                *active_tasks.entry(next_node_id.clone()).or_insert(0) += 1;
                                total_active_tasks += 1;
                                info!(
                                    "Task started: {}. Total active tasks: {}",
                                    &next_node_id, total_active_tasks
                                );

                                Self::worker(
                                    next_node_id,
                                    node_arc,
                                    ctx_arc,
                                    input_clone,
                                    tx_clone,
                                );
                            }
                        }
                    }
                }
                TaskEvent::Completed(node_id) => {
                    let mut is_empty = false;
                    let mut current_count = 0;
                    if let Some(count) = active_tasks.get_mut(&node_id) {
                        *count -= 1;
                        current_count = *count;
                        if *count == 0 {
                            is_empty = true;
                        }
                        total_active_tasks -= 1;
                    }

                    if is_empty {
                        active_tasks.remove(&node_id);
                    }

                    info!(
                        "Task completed: {}. Remaining active tasks for this node: {}. Total active: {}",
                        node_id, current_count, total_active_tasks
                    );
                }
                TaskEvent::Error(node_id, e) => {
                    error!("Node '{}' error: {}", node_id, e);
                    let mut is_empty = false;
                    if let Some(count) = active_tasks.get_mut(&node_id) {
                        *count -= 1;
                        if *count == 0 {
                            is_empty = true;
                        }
                        total_active_tasks -= 1;
                    }

                    if is_empty {
                        active_tasks.remove(&node_id);
                    }

                    info!(
                        "Task error exit: {}. Total active tasks: {}",
                        node_id, total_active_tasks
                    );
                }
            }

            if total_active_tasks == 0 {
                info!("All tasks finished, exiting runner loop.");
                break;
            }
        }

        // 如果没有初始任务，直接退出
        if total_active_tasks == 0 {
            return Ok(());
        }

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
                    // 如果输出是流，需要继续等待下一个输出
                    let is_stream = out.as_ref().is_stream();
                    let _ = tx.send(TaskEvent::Next(node_id.clone(), out)).await;
                    if !is_stream {
                        let _ = tx.send(TaskEvent::Completed(node_id)).await;
                    }
                }
                Err(e) => {
                    let _ = tx.send(TaskEvent::Error(node_id, e)).await;
                }
            }
        });
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
