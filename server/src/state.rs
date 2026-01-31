use crate::db::Db;
use crate::registry::NodeRegistry;
use async_trait::async_trait;
use flow::{
    AnyNode, Context, Edge as FlowEdge, Graph, Input, Output, RunnerHandle, SubgraphConfig,
    SubgraphExecutor,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex, RwLock as StdRwLock};
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

struct ExecutableSubgraphNode {
    executor: SubgraphExecutor,
}

#[async_trait]
impl flow::Node for ExecutableSubgraphNode {
    async fn run(&mut self, ctx: &Context, input: &Input) -> Result<Output, String> {
        let v = if let Some(v) = input.get_str("external_start") {
            v.clone()
        } else {
            let m = input.get_values();
            if m.is_empty() {
                Value::Null
            } else if m.len() == 1 {
                m.values().next().cloned().unwrap_or(Value::Null)
            } else {
                let mut obj = serde_json::Map::new();
                for (k, v) in m {
                    obj.insert(k.clone(), v.clone());
                }
                Value::Object(obj)
            }
        };

        let out = self
            .executor
            .execute(v, ctx)
            .await
            .map_err(|e| e.to_string())?;
        Ok(out.into())
    }
}

struct SubgraphStartNode;

#[async_trait]
impl flow::Node for SubgraphStartNode {
    async fn run(&mut self, _ctx: &Context, input: &Input) -> Result<Output, String> {
        Ok(input.get_any().cloned().unwrap_or(Value::Null).into())
    }
}

struct SubgraphInNode {
    key: String,
}

#[async_trait]
impl flow::Node for SubgraphInNode {
    async fn run(&mut self, _ctx: &Context, input: &Input) -> Result<Output, String> {
        let v = input.get_any().cloned().unwrap_or(Value::Null);
        if let Value::Object(map) = v {
            Ok(map.get(&self.key).cloned().unwrap_or(Value::Null).into())
        } else {
            Ok(v.into())
        }
    }
}

struct SubgraphEndNode;

#[async_trait]
impl flow::Node for SubgraphEndNode {
    async fn run(&mut self, _ctx: &Context, input: &Input) -> Result<Output, String> {
        let m = input.get_values();
        if m.is_empty() {
            return Ok(Value::Null.into());
        }
        if m.len() == 1 {
            return Ok(m.values().next().cloned().unwrap_or(Value::Null).into());
        }
        let mut obj = serde_json::Map::new();
        for (k, v) in m {
            obj.insert(k.clone(), v.clone());
        }
        Ok(Value::Object(obj).into())
    }
}

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

fn create_arc_node(
    registry: &NodeRegistry,
    node: &Node,
) -> Result<Arc<tokio::sync::RwLock<dyn AnyNode>>, String> {
    registry.create_node(&node.type_name, node_config_with_id(node))
}

fn build_subgraph_executor(
    group_node: &Node,
    member_nodes: &[Node],
    internal_edges: &[Edge],
    inbound_edges: &[Edge],
    outbound_edges: &[Edge],
    registry: &NodeRegistry,
    runtime_subgraphs: &HashMap<String, Arc<tokio::sync::RwLock<dyn AnyNode>>>,
) -> Result<SubgraphExecutor, String> {
    let group_id = &group_node.id;
    let start_id = format!("__subgraph_start__{}", group_id);
    let end_id = format!("__subgraph_end__{}", group_id);

    let mut g = Graph::new();

    // 构造子图的开始与结束节点
    g.add_node(&start_id, SubgraphStartNode);
    g.add_node(&end_id, SubgraphEndNode);

    for n in member_nodes {
        // 我们会从最内层的子图开始，逐步添加到子图中
        let arc = if n.type_name == SUBGRAPH_NODE_TYPE {
            runtime_subgraphs
                .get(&n.id)
                .cloned()
                .ok_or_else(|| format!("Nested subgraph '{}' has not been built yet", n.id))?
        } else {
            create_arc_node(registry, n)?
        };
        // 成员节点只要不是子图节点，都要添加到子图中
        g.add_arc_node(&n.id, arc, None);
    }

    // 处理「外部 -> 子图内部」的跨边：为每个外部 source 创建一个代理输入节点
    // 目的：把外部节点输出（在外层图里）打包成对象后，子图入口只接收一个 external_start；
    //      代理节点负责从 external_start 中按 source id 取出对应值，再喂给子图内部目标节点。
    let mut inbound_source_to_proxy: HashMap<String, String> = HashMap::new();
    for e in inbound_edges {
        if !inbound_source_to_proxy.contains_key(&e.source) {
            let proxy_id = format!("__subgraph_in__{}__{}", group_id, e.source);
            g.add_node(
                &proxy_id,
                SubgraphInNode {
                    key: e.source.clone(),
                },
            );
            g.add_edge(FlowEdge {
                from: start_id.clone(),
                to: proxy_id.clone(),
                condition: None,
            });
            inbound_source_to_proxy.insert(e.source.clone(), proxy_id);
        }
    }

    // 统计子图内部「有入边/有出边」的节点，用于自动补齐 start/end 的连线
    let mut has_incoming: HashSet<String> = HashSet::new();
    for e in internal_edges {
        has_incoming.insert(e.target.clone());
    }
    for e in inbound_edges {
        has_incoming.insert(e.target.clone());
    }

    let mut has_outgoing: HashSet<String> = HashSet::new();
    for e in internal_edges {
        has_outgoing.insert(e.source.clone());
    }

    // 子图内部连线去重：不同层折叠/多条边可能会生成同样的 (from,to)
    let mut seen_edges: HashSet<(String, String)> = HashSet::new();

    let add_edge =
        |from: String, to: String, g: &mut Graph, seen: &mut HashSet<(String, String)>| {
            if seen.insert((from.clone(), to.clone())) {
                g.add_edge(FlowEdge {
                    from,
                    to,
                    condition: None,
                });
            }
        };

    // 1) 子图内部原生连线：成员节点之间的边
    for e in internal_edges {
        add_edge(e.source.clone(), e.target.clone(), &mut g, &mut seen_edges);
    }

    // 2) 外部 -> 子图内部：外层 source 通过代理节点输入给内部 target
    // 把从外面连入的节点，转为代理节点，再连接到子图内部目标节点
    for e in inbound_edges {
        if let Some(proxy_id) = inbound_source_to_proxy.get(&e.source) {
            add_edge(proxy_id.clone(), e.target.clone(), &mut g, &mut seen_edges);
        }
    }

    // 3) 入口补边：对没有任何入边的内部节点，自动从 start 连过去，保证子图可启动
    for n in member_nodes {
        if !has_incoming.contains(&n.id) {
            add_edge(start_id.clone(), n.id.clone(), &mut g, &mut seen_edges);
        }
    }

    // 选择子图出口：
    // - 若存在「内部 -> 外部」跨边，则这些跨边的 source 作为出口候选
    // - 否则，以子图内部“无出边”的节点作为出口候选（末端节点）
    let mut exit_sources: HashSet<String> = HashSet::new();
    if !outbound_edges.is_empty() {
        for e in outbound_edges {
            exit_sources.insert(e.source.clone());
        }
    } else {
        for n in member_nodes {
            if !has_outgoing.contains(&n.id) {
                exit_sources.insert(n.id.clone());
            }
        }
    }

    // 将出口候选连接到 end；若没有任何候选，则 start -> end（空子图/无可达输出）
    if exit_sources.is_empty() {
        add_edge(start_id.clone(), end_id.clone(), &mut g, &mut seen_edges);
    } else {
        for src in exit_sources {
            add_edge(src, end_id.clone(), &mut g, &mut seen_edges);
        }
    }

    // 读取子图执行配置（来自 group 节点的 data.config）：
    // - inherit_context: 子图是否继承父 Context
    // - timeout_ms: 子图整体超时（毫秒）
    let inherit_context = group_node
        .data
        .config
        .get("inherit_context")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let timeout_ms = group_node
        .data
        .config
        .get("timeout_ms")
        .and_then(|v| v.as_u64());

    // 绑定子图的 entry/exit 节点，并构造 SubgraphExecutor 所需配置
    let mut config = SubgraphConfig::default();
    config.entry_node = start_id;
    config.exit_node = end_id;
    config.inherit_context = inherit_context;
    config.timeout = timeout_ms.map(|ms| std::time::Duration::from_millis(ms));

    Ok(SubgraphExecutor::new(g, config))
}

fn fold_subgraphs(
    nodes: &[Node],
    edges: &[Edge],
    registry: &NodeRegistry,
) -> Result<
    (
        Vec<Node>,
        Vec<Edge>,
        HashMap<String, Arc<tokio::sync::RwLock<dyn AnyNode>>>,
    ),
    String,
> {
    let mut nodes = nodes.to_vec();
    let mut edges = edges.to_vec();
    let mut runtime_subgraphs: HashMap<String, Arc<tokio::sync::RwLock<dyn AnyNode>>> =
        HashMap::new();

    // 收集所有的子图节点
    let mut group_ids: Vec<String> = nodes
        .iter()
        .filter(|n| n.type_name == SUBGRAPH_NODE_TYPE)
        .map(|n| n.id.clone())
        .collect();

    // 无子图直接返回
    if group_ids.is_empty() {
        return Ok((nodes, edges, runtime_subgraphs));
    }

    let group_id_set: HashSet<String> = group_ids.iter().cloned().collect();
    let mut group_parent: HashMap<String, String> = HashMap::new();
    for n in &nodes {
        if n.type_name == SUBGRAPH_NODE_TYPE {
            // 子图套子图的情况
            if let Some(pid) = n.parent_id.as_ref() {
                if group_id_set.contains(pid) {
                    group_parent.insert(n.id.clone(), pid.clone());
                }
            }
        }
    }

    // 判断每个子图的嵌套深度
    let mut depth_memo: HashMap<String, usize> = HashMap::new();
    let depth_of = |id: &String| -> usize {
        let mut cur = id.clone();
        let mut depth: usize = 0;
        let mut visited: HashSet<String> = HashSet::new();
        while let Some(p) = group_parent.get(&cur) {
            if !visited.insert(cur.clone()) {
                break;
            }
            depth += 1;
            cur = p.clone();
        }
        depth
    };

    for id in &group_ids {
        let d = depth_of(id);
        depth_memo.insert(id.clone(), d);
    }

    // 从嵌套最深的子图开始处理
    group_ids.sort_by_key(|id| std::cmp::Reverse(*depth_memo.get(id).unwrap_or(&0)));

    // 寻找每个子图的所有成员节点
    for group_id in group_ids {
        let group_node = match nodes.iter().find(|n| n.id == group_id) {
            Some(n) => n.clone(),
            None => continue,
        };

        // 收集子图的所有成员节点 Id 然后去重，然后 id -> Vec<Node>
        let member_ids: Vec<String> = nodes
            .iter()
            .filter(|n| n.parent_id.as_deref() == Some(group_node.id.as_str()))
            .map(|n| n.id.clone())
            .collect();

        let member_set: HashSet<String> = member_ids.iter().cloned().collect();

        let member_nodes: Vec<Node> = nodes
            .iter()
            .filter(|n| member_set.contains(&n.id))
            .cloned()
            .collect();

        // 收集子图的编
        let mut internal_edges = Vec::new();
        let mut inbound_edges = Vec::new();
        let mut outbound_edges = Vec::new();

        for e in &edges {
            let s_in = member_set.contains(&e.source);
            let t_in = member_set.contains(&e.target);
            // 子图的内部边
            if s_in && t_in {
                internal_edges.push(e.clone());
            }
            // 从外部连入的边
            else if !s_in && t_in {
                inbound_edges.push(e.clone());
            }
            // 从内连出去的边
            else if s_in && !t_in {
                outbound_edges.push(e.clone());
            }
        }

        let executor = build_subgraph_executor(
            &group_node,
            &member_nodes,
            &internal_edges,
            &inbound_edges,
            &outbound_edges,
            registry,
            &runtime_subgraphs,
        )?;
        let runtime = Arc::new(tokio::sync::RwLock::new(ExecutableSubgraphNode {
            executor,
        })) as Arc<tokio::sync::RwLock<dyn AnyNode>>;
        runtime_subgraphs.insert(group_node.id.clone(), runtime);

        nodes.retain(|n| !member_set.contains(&n.id));
        edges.retain(|e| !(member_set.contains(&e.source) || member_set.contains(&e.target)));

        let mut seen: HashSet<(String, String)> = edges
            .iter()
            .map(|e| (e.source.clone(), e.target.clone()))
            .collect();

        for e in inbound_edges {
            let key = (e.source.clone(), group_node.id.clone());
            if seen.insert(key) {
                edges.push(Edge {
                    id: format!("fold:{}:in:{}", group_node.id, e.id),
                    source: e.source,
                    target: group_node.id.clone(),
                    source_handle: e.source_handle,
                    target_handle: None,
                });
            }
        }

        for e in outbound_edges {
            let key = (group_node.id.clone(), e.target.clone());
            if seen.insert(key) {
                edges.push(Edge {
                    id: format!("fold:{}:out:{}", group_node.id, e.id),
                    source: group_node.id.clone(),
                    target: e.target,
                    source_handle: None,
                    target_handle: e.target_handle,
                });
            }
        }
    }

    Ok((nodes, edges, runtime_subgraphs))
}

impl GraphBlueprint {
    pub fn instantiate(&self, registry: &NodeRegistry) -> Result<(Graph, Vec<String>), String> {
        let (nodes, edges, runtime_subgraphs) = fold_subgraphs(&self.nodes, &self.edges, registry)?;
        let mut new_graph = Graph::new();
        let mut start_nodes = Vec::new();

        // 所有入度为 0 的节点都是起始节点
        let mut has_incoming_edges = HashSet::new();
        for edge in &edges {
            has_incoming_edges.insert(edge.target.clone());
        }

        for node in &nodes {
            let arc = if node.type_name == SUBGRAPH_NODE_TYPE {
                runtime_subgraphs
                    .get(&node.id)
                    .cloned()
                    .ok_or_else(|| format!("Subgraph '{}' not built", node.id))?
            } else {
                match registry.create_node(&node.type_name, node_config_with_id(node)) {
                    Ok(n) => n,
                    Err(e) => {
                        let msg = format!(
                            "Failed to create node {}({}): {}",
                            node.id, node.type_name, e
                        );
                        error!("{}", msg);
                        return Err(msg);
                    }
                }
            };

            new_graph.add_arc_node(&node.id, arc, None);
            if !has_incoming_edges.contains(&node.id) {
                start_nodes.push(node.id.clone());
            }
        }

        for edge in &edges {
            new_graph.add_edge(FlowEdge {
                from: edge.source.clone(),
                to: edge.target.clone(),
                condition: None,
            });
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

#[cfg(test)]
mod tests {
    use super::{Edge, GraphBlueprint, Node, NodeData, Position};
    use flow::{ExecutionContext, Runner};
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
        let registry = crate::registry::create_registry();

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
            }],
        };

        let (graph, start_nodes) = blueprint.instantiate(&registry).unwrap();

        assert!(graph.nodes.contains_key("SubgraphNode-1"));
        assert!(graph.nodes.contains_key("TextNode-2"));
        assert!(!graph.nodes.contains_key("TextNode-1"));

        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async move {
            let (mut runner, _handle) = Runner::new(graph, Some(ExecutionContext::new()));
            for id in start_nodes {
                runner.set_start_node(&id, Value::Null);
            }
            runner.run().await.unwrap();
            assert_eq!(
                runner.get_execution_context().get_output("TextNode-2"),
                Some(json!("inner"))
            );
        });
    }
}
