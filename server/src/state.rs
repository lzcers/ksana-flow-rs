use crate::db::Db;
use crate::registry::NodeRegistry;
use flow::{
    AnyNode, ControllerHandle, Graph, NodeFactory, RunnerHandle, RunnerId, SubgraphExecutor,
    SubgraphNode,
};
use nodes::SubgraphMapNode;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock as StdRwLock};
use tokio::sync::broadcast;
use tokio::{sync::RwLock, task::JoinHandle};
use tracing::error;

#[derive(Clone)]
pub struct ExecutionHandle {
    pub runner_handle: RunnerHandle,
    pub workflow_id: i64,
    pub workspace_id: String,
    pub controller: ControllerHandle,
    pub root_runner_id: RunnerId,
    pub tasks: Arc<tokio::sync::Mutex<ExecutionTaskHandles>>,
}

pub struct ExecutionTaskHandles {
    pub runner_task: Option<JoinHandle<()>>,
    pub bridge_task: Option<JoinHandle<()>>,
}

#[derive(Clone)]
pub struct AppState {
    // 所有节点的定义
    pub registry: Arc<NodeRegistry>,
    // 持有 Runner 的句柄
    pub executions: Arc<StdRwLock<HashMap<String, ExecutionHandle>>>,
    // (workspace_id, run_id, event)
    pub tx: broadcast::Sender<(String, String, Value)>,
    pub db: Arc<Mutex<Db>>,
}

#[derive(Serialize, Deserialize, Clone, Default, Debug)]
pub struct GraphBlueprint {
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
}

const SUBGRAPH_NODE_TYPE: &str = "SubgraphNode";
const MAP_NODE_TYPE: &str = "MapNode";

fn node_config_with_id(node: &Node) -> Value {
    let mut config = node.data.config.clone();
    if !config.is_object() {
        config = json!({});
    }
    if let Some(obj) = config.as_object_mut() {
        obj.insert("id".to_string(), Value::String(node.id.clone()));
    }
    config
}

impl GraphBlueprint {
    pub fn instantiate(
        &self,
        registry: Arc<NodeRegistry>,
    ) -> Result<(Arc<Graph>, Vec<String>), String> {
        let nodes: Vec<flow::BlueprintNode> = self
            .nodes
            .iter()
            .map(|n| flow::BlueprintNode {
                id: n.id.clone(),
                type_name: n.type_name.clone(),
                parent_id: n.parent_id.clone(),
                config: node_config_with_id(n),
            })
            .collect();

        let edges: Vec<flow::BlueprintEdge> = self
            .edges
            .iter()
            .map(|e| {
                let kind = match e.kind.as_deref() {
                    Some("data") => flow::EdgeKind::Data,
                    _ => flow::EdgeKind::Control,
                };
                let data_type = e.data_type.as_ref().and_then(|dt| match dt.as_str() {
                    "string" => Some(flow::DataType::String),
                    "number" => Some(flow::DataType::Number),
                    "boolean" => Some(flow::DataType::Boolean),
                    "json" => Some(flow::DataType::Json),
                    "binary" => Some(flow::DataType::Binary),
                    "any" => Some(flow::DataType::Any),
                    _ => None,
                });
                flow::BlueprintEdge {
                    id: e.id.clone(),
                    source: e.source.clone(),
                    target: e.target.clone(),
                    source_handle: e.source_handle.clone(),
                    target_handle: e.target_handle.clone(),
                    condition: e.condition.clone(),
                    kind,
                    source_port: e.source_port.clone(),
                    target_port: e.target_port.clone(),
                    data_type,
                }
            })
            .collect();

        let create_leaf_factory =
            move |node: &flow::BlueprintNode| -> Result<Arc<NodeFactory>, String> {
                if registry.get_node_metadata(&node.type_name).is_none() {
                    let msg = format!(
                        "Node type '{}' not found for node '{}'",
                        node.type_name, node.id
                    );
                    error!("{}", msg);
                    return Err(msg);
                }
                let type_name = node.type_name.clone();
                let config = node.config.clone();
                let registry = registry.clone();
                Ok(Arc::new(move || {
                    registry.create_node(&type_name, config.clone())
                }))
            };

        let is_group = |n: &flow::BlueprintNode| {
            n.type_name == SUBGRAPH_NODE_TYPE || n.type_name == MAP_NODE_TYPE
        };
        let create_group_factory = |group_node: &flow::BlueprintNode,
                                    executor: SubgraphExecutor|
         -> Result<Arc<NodeFactory>, String> {
            match group_node.type_name.as_str() {
                SUBGRAPH_NODE_TYPE => Ok(Arc::new(move || {
                    let node = SubgraphNode {
                        executor: executor.clone(),
                    };
                    Ok(Arc::new(RwLock::new(node)) as Arc<RwLock<dyn AnyNode>>)
                })),
                MAP_NODE_TYPE => {
                    let max_concurrency = group_node
                        .config
                        .get("max_concurrency")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(10) as usize;
                    let streaming = group_node
                        .config
                        .get("streaming")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);
                    Ok(Arc::new(move || {
                        let node = SubgraphMapNode::with_executor(
                            executor.clone(),
                            max_concurrency,
                            streaming,
                        );
                        Ok(Arc::new(RwLock::new(node)) as Arc<RwLock<dyn AnyNode>>)
                    }))
                }
                other => Err(format!("Unknown group node type: {}", other)),
            }
        };

        flow::compile_graph_with_groups(
            &nodes,
            &edges,
            is_group,
            create_group_factory,
            create_leaf_factory,
        )
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
    #[serde(rename = "parentId", skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extent: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hidden: Option<bool>,
    #[serde(default, flatten)]
    pub ui: HashMap<String, Value>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct NodeData {
    #[serde(default)]
    pub label: String,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub condition: Option<Value>,
    /// 边类型：控制流或数据流
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    /// 数据流边的源端口 ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_port: Option<String>,
    /// 数据流边的目标端口 ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_port: Option<String>,
    /// 数据流边的数据类型
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_type: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::{Edge, GraphBlueprint, Node, NodeData, Position};
    use flow::{Controller, ControllerRunners, ExecutionContext, RunnerKind};
    use serde_json::{Value, json};

    #[test]
    fn blueprint_preserves_grouping_fields() {
        let input = json!({
            "nodes": [
                {
                    "id": "child-1",
                    "type": "TextNode",
                    "data": {},
                    "position": { "x": 0.0, "y": 0.0 },
                    "parentId": "SubgraphNode-1",
                    "extent": "parent",
                    "hidden": true,
                    "expandParent": false
                }
            ],
            "edges": []
        });

        let blueprint: GraphBlueprint = serde_json::from_value(input).unwrap();
        let out = serde_json::to_value(&blueprint).unwrap();

        assert_eq!(out["nodes"][0]["parentId"], json!("SubgraphNode-1"));
        assert_eq!(out["nodes"][0]["extent"], json!("parent"));
        assert_eq!(out["nodes"][0]["hidden"], json!(true));
        assert_eq!(out["nodes"][0]["expandParent"], json!(false));
    }

    #[test]
    fn instantiate_folds_subgraph_and_runs() {
        let registry = std::sync::Arc::new(crate::registry::create_registry());

        let blueprint = GraphBlueprint {
            nodes: vec![
                Node {
                    id: "SubgraphNode-1".to_string(),
                    type_name: "SubgraphNode".to_string(),
                    data: NodeData {
                        label: "Group".to_string(),
                        config: json!({}),
                        ..Default::default()
                    },
                    position: Position { x: 0.0, y: 0.0 },
                    width: None,
                    height: None,
                    parent_id: None,
                    extent: None,
                    hidden: None,
                    ui: Default::default(),
                },
                Node {
                    id: "TextNode-1".to_string(),
                    type_name: "TextNode".to_string(),
                    data: NodeData {
                        label: "Inner".to_string(),
                        config: json!({"text":"inner"}),
                        ..Default::default()
                    },
                    position: Position { x: 0.0, y: 0.0 },
                    width: None,
                    height: None,
                    parent_id: Some("SubgraphNode-1".to_string()),
                    extent: Some(json!("parent")),
                    hidden: None,
                    ui: Default::default(),
                },
                Node {
                    id: "TextNode-2".to_string(),
                    type_name: "TextNode".to_string(),
                    data: NodeData {
                        label: "Outer".to_string(),
                        config: json!({"text":""}),
                        ..Default::default()
                    },
                    position: Position { x: 0.0, y: 0.0 },
                    width: None,
                    height: None,
                    parent_id: None,
                    extent: None,
                    hidden: None,
                    ui: Default::default(),
                },
            ],
            edges: vec![Edge {
                id: "e1".to_string(),
                source: "TextNode-1".to_string(),
                target: "TextNode-2".to_string(),
                source_handle: None,
                target_handle: None,
                condition: None,
                kind: None,
                source_port: None,
                target_port: None,
                data_type: None,
            }],
        };

        let (graph, start_nodes) = blueprint.instantiate(registry).unwrap();

        assert!(graph.nodes.contains_key("SubgraphNode-1"));
        assert!(graph.nodes.contains_key("TextNode-2"));
        assert!(!graph.nodes.contains_key("TextNode-1"));

        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async move {
            let (controller, _rx) = Controller::new();
            let (_id, mut runner, _handle) = controller.create_runner(
                graph,
                Some(ExecutionContext::new()),
                RunnerKind::Root,
                None,
                None,
            );
            for id in start_nodes {
                runner.set_start_node(&id, Value::Null.into());
            }
            runner.run().await.unwrap();
            assert_eq!(
                runner.get_execution_context().get_output("TextNode-2"),
                Some(json!("inner"))
            );
        });
    }
}
