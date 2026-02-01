use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;

use super::{
    graph::{AnyNode, Edge as FlowEdge, EdgeCondition, Graph, NodeFactory},
    io::Output,
    node::{SubgraphEndNode, SubgraphInNode, SubgraphNode, SubgraphStartNode},
    runtime_context::Context,
    {SubgraphConfig, SubgraphExecutor},
};

#[derive(Clone, Debug)]
pub struct BlueprintNode {
    pub id: String,
    pub type_name: String,
    pub parent_id: Option<String>,
    pub config: Value,
}

#[derive(Clone, Debug)]
pub struct BlueprintEdge {
    pub id: String,
    pub source: String,
    pub target: String,
    pub source_handle: Option<String>,
    pub target_handle: Option<String>,
    pub condition: Option<Value>,
}

fn factory_from_creator<N, F>(creator: F) -> Arc<NodeFactory>
where
    N: AnyNode,
    F: Fn() -> N + Send + Sync + 'static,
{
    Arc::new(move || Ok(Arc::new(RwLock::new(creator())) as Arc<RwLock<dyn AnyNode>>))
}

fn parse_edge_condition(condition: Option<Value>) -> Result<Option<EdgeCondition>, String> {
    match condition {
        None => Ok(None),
        Some(Value::Bool(b)) => Ok(Some(Box::new(move |_ctx: &Context, _out: &Output| b))),
        Some(other) => Err(format!("Unsupported edge condition: {}", other)),
    }
}

fn build_subgraph_executor<F>(
    group_node: &BlueprintNode,
    member_nodes: &[BlueprintNode],
    internal_edges: &[BlueprintEdge],
    inbound_edges: &[BlueprintEdge],
    outbound_edges: &[BlueprintEdge],
    group_type: &str,
    create_leaf_factory: &F,
    runtime_subgraphs: &HashMap<String, Arc<NodeFactory>>,
) -> Result<SubgraphExecutor, String>
where
    F: Fn(&BlueprintNode) -> Result<Arc<NodeFactory>, String> + Clone,
{
    let group_id = &group_node.id;
    let start_id = format!("__subgraph_start__{}", group_id);
    let end_id = format!("__subgraph_end__{}", group_id);

    let mut g = Graph::new();
    g.add_node(&start_id, || SubgraphStartNode);
    g.add_node(&end_id, || SubgraphEndNode);

    for n in member_nodes {
        let factory = if n.type_name == group_type {
            runtime_subgraphs
                .get(&n.id)
                .cloned()
                .ok_or_else(|| format!("Nested subgraph '{}' has not been built yet", n.id))?
        } else {
            create_leaf_factory(n)?
        };
        g.add_node_factory(&n.id, factory);
    }

    let mut inbound_source_to_proxy: HashMap<String, String> = HashMap::new();
    for e in inbound_edges {
        if !inbound_source_to_proxy.contains_key(&e.source) {
            let proxy_id = format!("__subgraph_in__{}__{}", group_id, e.source);
            let key = e.source.clone();
            g.add_node(&proxy_id, move || SubgraphInNode { key: key.clone() });
            g.add_edge(FlowEdge {
                from: start_id.clone(),
                to: proxy_id.clone(),
                condition: None,
            });
            inbound_source_to_proxy.insert(e.source.clone(), proxy_id);
        }
    }

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

    let mut seen_edges: HashSet<(String, String)> = HashSet::new();
    let mut add_edge = |from: String, to: String, condition: Option<EdgeCondition>| {
        if seen_edges.insert((from.clone(), to.clone())) {
            g.add_edge(FlowEdge {
                from,
                to,
                condition,
            });
        }
    };

    for e in internal_edges {
        let condition = parse_edge_condition(e.condition.clone())?;
        add_edge(e.source.clone(), e.target.clone(), condition);
    }
    for e in inbound_edges {
        if let Some(proxy_id) = inbound_source_to_proxy.get(&e.source) {
            add_edge(proxy_id.clone(), e.target.clone(), None);
        }
    }
    for n in member_nodes {
        if !has_incoming.contains(&n.id) {
            add_edge(start_id.clone(), n.id.clone(), None);
        }
    }

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

    if exit_sources.is_empty() {
        add_edge(start_id.clone(), end_id.clone(), None);
    } else {
        for src in exit_sources {
            add_edge(src, end_id.clone(), None);
        }
    }

    let inherit_context = group_node
        .config
        .get("inherit_context")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let timeout_ms = group_node.config.get("timeout_ms").and_then(|v| v.as_u64());

    let mut config = SubgraphConfig::default();
    config.entry_node = start_id;
    config.exit_node = end_id;
    config.inherit_context = inherit_context;
    config.timeout = timeout_ms.map(std::time::Duration::from_millis);

    Ok(SubgraphExecutor::new(g, config))
}

fn fold_subgraphs<F>(
    nodes: &[BlueprintNode],
    edges: &[BlueprintEdge],
    group_type: &str,
    create_leaf_factory: &F,
) -> Result<
    (
        Vec<BlueprintNode>,
        Vec<BlueprintEdge>,
        HashMap<String, Arc<NodeFactory>>,
    ),
    String,
>
where
    F: Fn(&BlueprintNode) -> Result<Arc<NodeFactory>, String> + Clone,
{
    let mut nodes = nodes.to_vec();
    let mut edges = edges.to_vec();
    let mut runtime_subgraphs: HashMap<String, Arc<NodeFactory>> = HashMap::new();

    let mut group_ids: Vec<String> = nodes
        .iter()
        .filter(|n| n.type_name == group_type)
        .map(|n| n.id.clone())
        .collect();

    if group_ids.is_empty() {
        return Ok((nodes, edges, runtime_subgraphs));
    }

    let group_id_set: HashSet<String> = group_ids.iter().cloned().collect();
    let mut group_parent: HashMap<String, String> = HashMap::new();
    for n in &nodes {
        if n.type_name == group_type {
            if let Some(pid) = n.parent_id.as_ref() {
                if group_id_set.contains(pid) {
                    group_parent.insert(n.id.clone(), pid.clone());
                }
            }
        }
    }

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

    group_ids.sort_by_key(|id| std::cmp::Reverse(depth_of(id)));

    for group_id in group_ids {
        let group_node = match nodes.iter().find(|n| n.id == group_id) {
            Some(n) => n.clone(),
            None => continue,
        };

        let member_ids: Vec<String> = nodes
            .iter()
            .filter(|n| n.parent_id.as_deref() == Some(group_node.id.as_str()))
            .map(|n| n.id.clone())
            .collect();

        let member_set: HashSet<String> = member_ids.iter().cloned().collect();
        let member_nodes: Vec<BlueprintNode> = nodes
            .iter()
            .filter(|n| member_set.contains(&n.id))
            .cloned()
            .collect();

        let mut internal_edges = Vec::new();
        let mut inbound_edges = Vec::new();
        let mut outbound_edges = Vec::new();

        for e in &edges {
            let s_in = member_set.contains(&e.source);
            let t_in = member_set.contains(&e.target);
            if s_in && t_in {
                internal_edges.push(e.clone());
            } else if !s_in && t_in {
                inbound_edges.push(e.clone());
            } else if s_in && !t_in {
                outbound_edges.push(e.clone());
            }
        }

        let executor = build_subgraph_executor(
            &group_node,
            &member_nodes,
            &internal_edges,
            &inbound_edges,
            &outbound_edges,
            group_type,
            create_leaf_factory,
            &runtime_subgraphs,
        )?;

        let group_factory = factory_from_creator(move || SubgraphNode {
            executor: executor.clone(),
        });
        runtime_subgraphs.insert(group_node.id.clone(), group_factory);

        nodes.retain(|n| !member_set.contains(&n.id));
        edges.retain(|e| !(member_set.contains(&e.source) || member_set.contains(&e.target)));

        let mut seen: HashSet<(String, String)> = edges
            .iter()
            .map(|e| (e.source.clone(), e.target.clone()))
            .collect();

        for e in inbound_edges {
            let key = (e.source.clone(), group_node.id.clone());
            if seen.insert(key) {
                edges.push(BlueprintEdge {
                    id: format!("fold:{}:in:{}", group_node.id, e.id),
                    source: e.source,
                    target: group_node.id.clone(),
                    source_handle: e.source_handle,
                    target_handle: None,
                    condition: e.condition,
                });
            }
        }

        for e in outbound_edges {
            let key = (group_node.id.clone(), e.target.clone());
            if seen.insert(key) {
                edges.push(BlueprintEdge {
                    id: format!("fold:{}:out:{}", group_node.id, e.id),
                    source: group_node.id.clone(),
                    target: e.target,
                    source_handle: None,
                    target_handle: e.target_handle,
                    condition: e.condition,
                });
            }
        }
    }

    Ok((nodes, edges, runtime_subgraphs))
}

pub fn compile_graph<F>(
    nodes: &[BlueprintNode],
    edges: &[BlueprintEdge],
    group_type: &str,
    create_leaf_factory: F,
) -> Result<(Arc<Graph>, Vec<String>), String>
where
    F: Fn(&BlueprintNode) -> Result<Arc<NodeFactory>, String> + Clone,
{
    let (nodes, edges, runtime_subgraphs) =
        fold_subgraphs(nodes, edges, group_type, &create_leaf_factory)?;

    let mut new_graph = Graph::new();
    let mut start_nodes = Vec::new();

    let mut has_incoming_edges = HashSet::new();
    for edge in &edges {
        has_incoming_edges.insert(edge.target.clone());
    }

    for node in &nodes {
        let factory = if node.type_name == group_type {
            runtime_subgraphs
                .get(&node.id)
                .cloned()
                .ok_or_else(|| format!("Subgraph '{}' not built", node.id))?
        } else {
            create_leaf_factory(node)?
        };
        new_graph.add_node_factory(&node.id, factory);
        if !has_incoming_edges.contains(&node.id) {
            start_nodes.push(node.id.clone());
        }
    }

    for edge in &edges {
        let condition = parse_edge_condition(edge.condition.clone())?;
        new_graph.add_edge(FlowEdge {
            from: edge.source.clone(),
            to: edge.target.clone(),
            condition,
        });
    }

    Ok((Arc::new(new_graph), start_nodes))
}
