use crate::db::Db;
use crate::registry::NodeRegistry;
use flow::{AnyEdge, Edge as FlowEdge, FlowEvent, Graph, RunnerHandle};
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
}

#[derive(Clone)]
pub struct AppState {
    pub registry: Arc<RwLock<NodeRegistry>>,
    pub executions: Arc<RwLock<HashMap<String, ExecutionHandle>>>,
    pub tx: broadcast::Sender<(String, FlowEvent)>,
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

        // Collect all target nodes (nodes that have incoming edges)
        let mut has_incoming_edges = HashSet::new();
        for edge in &self.edges {
            has_incoming_edges.insert(edge.target.clone());
        }

        for node in &self.nodes {
            let mut data = node.data.clone();
            if let Some(obj) = data.as_object_mut() {
                obj.insert("id".to_string(), Value::String(node.id.clone()));
            }

            match registry.create_node(&node.type_name, data) {
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
                let edge_instance: Option<Box<dyn AnyEdge>> = match type_name.as_str() {
                    "ReactiveSourceNode" => Some(Box::new(FlowEdge::<K> {
                        from: edge.source.clone(),
                        to: edge.target.clone(),
                        condition: None,
                    })),
                    "VOLMFINode" => Some(Box::new(FlowEdge::<BacktesterInput> {
                        from: edge.source.clone(),
                        to: edge.target.clone(),
                        condition: None,
                    })),
                    "Backtester" => Some(Box::new(FlowEdge::<()> {
                        from: edge.source.clone(),
                        to: edge.target.clone(),
                        condition: None,
                    })),
                    "TextNode" => Some(Box::new(FlowEdge::<String> {
                        from: edge.source.clone(),
                        to: edge.target.clone(),
                        condition: None,
                    })),
                    "LLMNode" => Some(Box::new(FlowEdge::<String> {
                        from: edge.source.clone(),
                        to: edge.target.clone(),
                        condition: None,
                    })),
                    "TextFileNode" => Some(Box::new(FlowEdge::<String> {
                        from: edge.source.clone(),
                        to: edge.target.clone(),
                        condition: None,
                    })),
                    _ => None,
                };
                if let Some(e) = edge_instance {
                    new_graph
                        .edges
                        .entry(edge.source.clone())
                        .or_insert_with(Vec::new)
                        .push(e);
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
    pub data: Value,
    pub position: Position,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<f64>,
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
