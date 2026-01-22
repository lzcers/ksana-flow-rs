use crate::db::Db;
use crate::registry::NodeRegistry;
use flow::{Edge as FlowEdge, FlowEvent, Graph, RunnerHandle};
use nodes::trade::K;
use nodes::trade::backtester::BacktesterInput;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex, RwLock};
use tokio::sync::broadcast;
use tracing::error;

#[derive(Clone)]
pub struct ExecutionHandle {
    pub runner_handle: RunnerHandle,
    pub workflow_id: i64,
    pub workspace_id: String,
}

#[derive(Clone)]
pub struct AppState {
    // 所有节点的定义
    pub registry: Arc<NodeRegistry>,
    // 持有 Runner 的句柄
    pub executions: Arc<RwLock<HashMap<String, ExecutionHandle>>>,
    // (workspace_id, run_id, event)
    pub tx: broadcast::Sender<(String, String, FlowEvent)>,
    pub db: Arc<Mutex<Db>>,
}

#[derive(Serialize, Deserialize, Clone, Default, Debug)]
pub struct GraphBlueprint {
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
}

impl GraphBlueprint {
    pub fn instantiate(&self, registry: &NodeRegistry) -> Result<(Graph, Vec<String>), String> {
        let mut new_graph = Graph::new();
        let mut start_nodes = Vec::new();

        // 所有入度为 0 的节点都是起始节点
        let mut has_incoming_edges = HashSet::new();
        for edge in &self.edges {
            has_incoming_edges.insert(edge.target.clone());
        }

        for node in &self.nodes {
            let mut config = node.data.config.clone();
            // Ensure config is an object so we can insert id
            if !config.is_object() {
                config = serde_json::json!({});
            }

            if let Some(obj) = config.as_object_mut() {
                obj.insert("id".to_string(), Value::String(node.id.clone()));
            }

            match registry.create_node(&node.type_name, config) {
                Ok(n) => {
                    new_graph.add_arc_node(&node.id, n);
                    // Treat all nodes with no in-degree as start nodes
                    if !has_incoming_edges.contains(&node.id) {
                        start_nodes.push(node.id.clone());
                    }
                }
                Err(e) => {
                    let msg = format!(
                        "Failed to create node {}({}): {}",
                        node.id, node.type_name, e
                    );
                    error!("{}", msg);
                    return Err(msg);
                }
            }
        }

        for edge in &self.edges {
            let source_node = self.nodes.iter().find(|n| n.id == edge.source);
            if let Some(source_node) = source_node {
                let type_name = &source_node.type_name;
                match type_name.as_str() {
                    "ReactiveSourceNode" => new_graph.add_edge(FlowEdge::<K> {
                        from: edge.source.clone(),
                        to: edge.target.clone(),
                        condition: None,
                    }),
                    "VOLMFINode" => new_graph.add_edge(FlowEdge::<BacktesterInput> {
                        from: edge.source.clone(),
                        to: edge.target.clone(),
                        condition: None,
                    }),
                    "Backtester" => new_graph.add_edge(FlowEdge::<()> {
                        from: edge.source.clone(),
                        to: edge.target.clone(),
                        condition: None,
                    }),
                    "TextNode" => new_graph.add_edge(FlowEdge::<String> {
                        from: edge.source.clone(),
                        to: edge.target.clone(),
                        condition: None,
                    }),
                    "LLMNode" => new_graph.add_edge(FlowEdge::<String> {
                        from: edge.source.clone(),
                        to: edge.target.clone(),
                        condition: None,
                    }),
                    "TextFileNode" => new_graph.add_edge(FlowEdge::<String> {
                        from: edge.source.clone(),
                        to: edge.target.clone(),
                        condition: None,
                    }),
                    "StreamLLMNode" => new_graph.add_edge(FlowEdge::<String> {
                        from: edge.source.clone(),
                        to: edge.target.clone(),
                        condition: None,
                    }),
                    "TextMergeNode" => new_graph.add_edge(FlowEdge::<String> {
                        from: edge.source.clone(),
                        to: edge.target.clone(),
                        condition: None,
                    }),
                    _ => {}
                }
            }
        }
        Ok((new_graph, start_nodes))
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Node {
    pub id: String,
    #[serde(rename = "type")]
    pub type_name: String,
    pub data: NodeData,
    pub position: Position,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<f64>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct NodeData {
    #[serde(default)]
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default)]
    pub config: Value,
    #[serde(default)]
    pub inputs: Value,
    #[serde(default)]
    pub outputs: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(rename = "errorMessage", skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    #[serde(rename = "lastMessage", skip_serializing_if = "Option::is_none")]
    pub last_message: Option<Value>,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Position {
    pub x: f64,
    pub y: f64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Edge {
    pub id: String,
    pub source: String,
    pub target: String,
    #[serde(rename = "sourceHandle")]
    pub source_handle: Option<String>,
    #[serde(rename = "targetHandle")]
    pub target_handle: Option<String>,
}
