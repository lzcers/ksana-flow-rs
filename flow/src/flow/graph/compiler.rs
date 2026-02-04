//! 子图（Subgraph）蓝图编译器。
//!
//! 作用：把带有 `parent_id` 分组关系的蓝图（`BlueprintNode/BlueprintEdge`）折叠成可执行的 `Graph`。
//! - `group_type` 用来标记“子图容器节点”的 type_name（例如 `SubgraphNode`）。容器节点本身会被编译为一个运行时子图节点。
//! - 容器节点的子节点（`parent_id == group_id`）会进入子图内部图。
//! - 子图支持嵌套：先编译最深层，再逐层向外折叠。
//! - 边的 `condition` 目前只支持布尔值；不支持的表达会在编译期报错，避免静默丢失条件语义。

use super::{
    graph::{AnyNode, Edge as FlowEdge, EdgeCondition, Graph, NodeFactory},
    io::Output,
    node::{SubgraphEndNode, SubgraphInNode, SubgraphNode, SubgraphStartNode},
    {SubgraphConfig, SubgraphExecutor},
};
use crate::Context;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone, Debug)]
/// 蓝图节点：仅描述“节点是什么/属于谁/有哪些配置”，不包含可执行状态。
///
/// - `id`：节点唯一标识
/// - `type_name`：节点类型名（用于从注册表/工厂创建运行时节点）
/// - `parent_id`：若为 Some，则表示该节点属于某个 group（子图容器）内部
/// - `config`：节点配置（从外部 JSON 透传到节点构造/运行时）
pub struct BlueprintNode {
    pub id: String,
    pub type_name: String,
    pub parent_id: Option<String>,
    pub config: Value,
}

#[derive(Clone, Debug)]
/// 蓝图边：连接两个蓝图节点。
///
/// - `condition`：可选条件，决定是否允许从 `source` 走到 `target`。
///   当前仅支持 `bool`，未来可以扩展为表达式/脚本等更复杂的条件类型。
pub struct BlueprintEdge {
    pub id: String,
    pub source: String,
    pub target: String,
    pub source_handle: Option<String>,
    pub target_handle: Option<String>,
    pub condition: Option<Value>,
}

// 将“无状态创建函数”包装为 Graph 需要的 NodeFactory：每次执行都生成全新的节点实例。
fn factory_from_creator<N, F>(creator: F) -> Arc<NodeFactory>
where
    N: AnyNode,
    F: Fn() -> N + Send + Sync + 'static,
{
    Arc::new(move || Ok(Arc::new(RwLock::new(creator())) as Arc<RwLock<dyn AnyNode>>))
}

// 解析蓝图边的 condition 到运行时可执行条件。
// 注意：为了保证语义保真，不支持的 condition 会直接返回错误，而不是忽略。
fn parse_edge_condition(condition: Option<Value>) -> Result<Option<EdgeCondition>, String> {
    match condition {
        None => Ok(None),
        Some(Value::Bool(b)) => Ok(Some(Box::new(move |_ctx: &Context, _out: &Output| b))),
        Some(other) => Err(format!("Unsupported edge condition: {}", other)),
    }
}

fn build_subgraph_executor<F, IsGroup>(
    group_node: &BlueprintNode,
    member_nodes: &[BlueprintNode],
    internal_edges: &[BlueprintEdge],
    inbound_edges: &[BlueprintEdge],
    outbound_edges: &[BlueprintEdge],
    is_group: &IsGroup,
    create_leaf_factory: &F,
    runtime_groups: &HashMap<String, Arc<NodeFactory>>,
) -> Result<SubgraphExecutor, String>
where
    F: Fn(&BlueprintNode) -> Result<Arc<NodeFactory>, String> + Clone,
    IsGroup: Fn(&BlueprintNode) -> bool,
{
    let group_id = &group_node.id;
    // 每个子图都会生成一对独立的入口/出口节点，作为内部执行的统一起点与汇点。
    let start_id = format!("__subgraph_start__{}", group_id);
    let end_id = format!("__subgraph_end__{}", group_id);

    let mut g = Graph::new();
    g.add_node(&start_id, || SubgraphStartNode);
    g.add_node(&end_id, || SubgraphEndNode);

    // 添加成员节点：普通节点由 create_leaf_factory 创建；若成员节点本身也是子图容器，则复用已编译好的 runtime_groups。
    for n in member_nodes {
        let factory = if is_group(n) {
            runtime_groups
                .get(&n.id)
                .cloned()
                .ok_or_else(|| format!("Nested group '{}' has not been built yet", n.id))?
        } else {
            create_leaf_factory(n)?
        };
        g.add_node_factory(&n.id, factory);
    }

    // 处理外部 -> 子图内部的跨边：为每个外部 source 创建一个代理输入节点。
    // 子图运行时只从 start 接收统一入口，代理节点负责按 source id 从入口数据里取出对应值，再喂给内部目标节点。
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

    // 统计内部节点是否存在入边/出边，用于后续补齐 start/end 的连线，保证子图可启动且可收敛到出口。
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

    // 子图内部边去重：折叠/补边过程中可能产生重复的 (from,to)。
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

    // 外部 -> 内部：由代理节点转接，跨图边不在子图内部直接出现。
    for e in inbound_edges {
        if let Some(proxy_id) = inbound_source_to_proxy.get(&e.source) {
            add_edge(proxy_id.clone(), e.target.clone(), None);
        }
    }

    // 入口补边：没有任何入边的节点由 start 直接触发，避免“子图启动后无可执行节点”。
    for n in member_nodes {
        if !has_incoming.contains(&n.id) {
            add_edge(start_id.clone(), n.id.clone(), None);
        }
    }

    // 出口选择：
    // - 若存在内部 -> 外部跨边，则这些跨边的 source 作为出口候选；
    // - 否则，选择内部图里“无出边”的末端节点作为出口候选。
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

    // 将出口候选统一连到 end；若没有候选（例如空子图），则 start -> end。
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

    // 子图执行配置由 group 节点的 config 决定：
    // - inherit_context：是否继承父 Context（通常通过 overlay 实现）
    // - timeout_ms：子图整体超时
    let mut config = SubgraphConfig::default();
    config.entry_node = start_id;
    config.exit_node = end_id;
    config.inherit_context = inherit_context;
    config.timeout = timeout_ms.map(std::time::Duration::from_millis);

    Ok(SubgraphExecutor::new(g, config))
}

fn fold_groups<F, IsGroup, CreateGroupFactory>(
    nodes: &[BlueprintNode],
    edges: &[BlueprintEdge],
    is_group: &IsGroup,
    create_group_factory: &CreateGroupFactory,
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
    IsGroup: Fn(&BlueprintNode) -> bool,
    CreateGroupFactory: Fn(&BlueprintNode, SubgraphExecutor) -> Result<Arc<NodeFactory>, String>,
{
    let mut nodes = nodes.to_vec();
    let mut edges = edges.to_vec();
    let mut runtime_groups: HashMap<String, Arc<NodeFactory>> = HashMap::new();

    let mut group_ids: Vec<String> = nodes
        .iter()
        .filter(|n| is_group(n))
        .map(|n| n.id.clone())
        .collect();

    if group_ids.is_empty() {
        return Ok((nodes, edges, runtime_groups));
    }

    let group_id_set: HashSet<String> = group_ids.iter().cloned().collect();
    let mut group_parent: HashMap<String, String> = HashMap::new();
    for n in &nodes {
        if is_group(n) {
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
            is_group,
            create_leaf_factory,
            &runtime_groups,
        )?;

        let group_factory = create_group_factory(&group_node, executor.clone())?;
        runtime_groups.insert(group_node.id.clone(), group_factory);

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

    Ok((nodes, edges, runtime_groups))
}

pub fn compile_graph_with_groups<F, IsGroup, CreateGroupFactory>(
    nodes: &[BlueprintNode],
    edges: &[BlueprintEdge],
    is_group: IsGroup,
    create_group_factory: CreateGroupFactory,
    create_leaf_factory: F,
) -> Result<(Arc<Graph>, Vec<String>), String>
where
    F: Fn(&BlueprintNode) -> Result<Arc<NodeFactory>, String> + Clone,
    IsGroup: Fn(&BlueprintNode) -> bool,
    CreateGroupFactory: Fn(&BlueprintNode, SubgraphExecutor) -> Result<Arc<NodeFactory>, String>,
{
    let (nodes, edges, runtime_groups) = fold_groups(
        nodes,
        edges,
        &is_group,
        &create_group_factory,
        &create_leaf_factory,
    )?;

    let mut new_graph = Graph::new();
    let mut start_nodes = Vec::new();

    let node_id_set: HashSet<&str> = nodes.iter().map(|n| n.id.as_str()).collect();
    let edges: Vec<&BlueprintEdge> = edges
        .iter()
        .filter(|e| node_id_set.contains(e.source.as_str()) && node_id_set.contains(e.target.as_str()))
        .collect();

    let mut has_incoming_edges = HashSet::new();
    for edge in &edges {
        has_incoming_edges.insert(edge.target.clone());
    }

    for node in &nodes {
        let factory = if is_group(node) {
            runtime_groups
                .get(&node.id)
                .cloned()
                .ok_or_else(|| format!("Group '{}' not built", node.id))?
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

/// 编译蓝图为可执行 `Graph`。
///
/// 返回：`(Arc<Graph>, start_nodes)`，其中 start_nodes 是入度为 0 的节点集合（作为默认起点）。
/// 注意：节点实例由 NodeFactory 延迟创建，保证每次 Runner 执行都拿到“本次运行专属”的节点对象。
pub fn compile_graph<F>(
    nodes: &[BlueprintNode],
    edges: &[BlueprintEdge],
    group_type: &str,
    create_leaf_factory: F,
) -> Result<(Arc<Graph>, Vec<String>), String>
where
    F: Fn(&BlueprintNode) -> Result<Arc<NodeFactory>, String> + Clone,
{
    let group_type = group_type.to_string();
    let is_group = move |n: &BlueprintNode| n.type_name == group_type;
    let create_group_factory = |_group_node: &BlueprintNode,
                                executor: SubgraphExecutor|
     -> Result<Arc<NodeFactory>, String> {
        Ok(factory_from_creator(move || SubgraphNode {
            executor: executor.clone(),
        }))
    };

    compile_graph_with_groups(
        nodes,
        edges,
        is_group,
        create_group_factory,
        create_leaf_factory,
    )
}
