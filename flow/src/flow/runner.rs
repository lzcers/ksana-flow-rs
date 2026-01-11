use super::{AnyNode, CloneAny, Context, Graph, NodeId, ReactiveStream, TaskResult};
use std::collections::VecDeque;
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
            graph: graph,
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
        let (tx, mut rx) = mpsc::channel::<TaskResult>(100);

        loop {
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
                    Self::worker(node_id, node_arc, ctx_arc, input_clone, tx_clone);
                }
            }
            if let Some(task_result) = rx.recv().await {
                match task_result {
                    Ok((node_id, output)) => {
                        let t_name = output.as_ref().type_name();
                        // 如果节点返回的是一个 ReactiveStream，
                        // 则在一个新的 tokio 任务中订阅它
                        if t_name.contains("ReactiveStream") {
                            let stream_box: Box<ReactiveStream> = output
                                .into_any()
                                .downcast::<ReactiveStream>()
                                .map_err(|_| "Failed to downcast to ReactiveStream".to_string())?;

                            let _sub =
                                (stream_box.subscribe)(tx.clone(), node_id, self.ctx.clone());
                            continue;
                        }

                        // Normal output handling
                        let next_nodes = self.find_next_nodes(&node_id, &output)?;
                        if !next_nodes.is_empty() {
                            self.task_queue.push_back((next_nodes, output));
                        }
                    }
                    Err(e) => {
                        error!("Task error: {}", e);
                        return Err(e);
                    }
                }
            } else {
                break;
            }
        }
        Ok(())
    }
    fn worker(
        node_id: String,
        node: Arc<RwLock<dyn AnyNode>>,
        ctx: Arc<Context>,
        input: Box<dyn CloneAny>,
        tx: Sender<TaskResult>,
    ) {
        tokio::spawn(async move {
            let mut node = node.write().await;
            let output = node
                .run(&ctx, input)
                .await
                .map_err(|e| format!("Node '{}' run error: {}", &node_id, e));
            info!(
                "Running node: <{}> in task: {:?}",
                &node_id,
                tokio::task::id(),
            );
            let result = output.map(|out| (node_id.clone(), out));
            if let Err(e) = tx.send(result).await {
                error!("Error sending result for node {}: {:?}", &node_id, e);
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
