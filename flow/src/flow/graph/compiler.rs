//! 子图（Subgraph）蓝图编译器。
//!
//! 作用：把带有 `parent_id` 分组关系的蓝图（`BlueprintNode/BlueprintEdge`）折叠成可执行的 `Graph`。
//! - `group_type` 用来标记"子图容器节点"的 type_name（例如 `SubgraphNode`）。容器节点本身会被编译为一个运行时子图节点。
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
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;

/// 蓝图节点：仅描述"节点是什么/属于谁/有哪些配置"，不包含可执行状态。
///
/// - `id`：节点唯一标识
/// - `type_name`：节点类型名（用于从注册表/工厂创建运行时节点）
/// - `parent_id`：若为 Some，则表示该节点属于某个 group（子图容器）内部
/// - `config`：节点配置（从外部 JSON 透传到节点构造/运行时）
#[derive(Clone, Debug)]
pub struct BlueprintNode {
    pub id: String,
    pub type_name: String,
    pub parent_id: Option<String>,
    pub config: Value,
}

/// 边类型：控制流或数据流
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EdgeKind {
    #[default]
    Control,
    Data,
}

/// 数据类型：用于数据流边的类型标注
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DataType {
    #[default]
    String,
    Number,
    Boolean,
    Json,
    Binary,
    Any,
}

/// 蓝图边：连接两个蓝图节点。
///
/// - `condition`：可选条件，决定是否允许从 `source` 走到 `target`。
///   当前仅支持 `bool`，未来可以扩展为表达式/脚本等更复杂的条件类型。
/// - `kind`：边类型，区分控制流边和数据流边。
/// - `source_port`/`target_port`：数据流边的源/目标端口 ID。
/// - `data_type`：数据流边的数据类型。
#[derive(Clone, Debug)]
pub struct BlueprintEdge {
    pub id: String,
    pub source: String,
    pub target: String,
    pub source_handle: Option<String>,
    pub target_handle: Option<String>,
    pub condition: Option<Value>,
    /// 边类型：控制流或数据流
    pub kind: EdgeKind,
    /// 数据流边的源端口 ID
    pub source_port: Option<String>,
    /// 数据流边的目标端口 ID
    pub target_port: Option<String>,
    /// 数据流边的数据类型
    pub data_type: Option<DataType>,
}

/// 边分类结果：将边按相对于子图成员的关系分为三类。
///
/// - `internal`：内部边，source 和 target 都在子图内
/// - `inbound`：入边，source 在外部，target 在子图内
/// - `outbound`：出边，source 在子图内，target 在外部
#[derive(Clone)]
struct ClassifiedEdges {
    internal: Vec<BlueprintEdge>,
    inbound: Vec<BlueprintEdge>,
    outbound: Vec<BlueprintEdge>,
}

impl ClassifiedEdges {
    /// 根据成员节点集合对边进行分类。
    ///
    /// # 参数
    /// - `edges`：待分类的边集合
    /// - `member_set`：子图成员节点 ID 集合
    fn classify(edges: &[BlueprintEdge], member_set: &HashSet<String>) -> Self {
        let mut internal = Vec::new();
        let mut inbound = Vec::new();
        let mut outbound = Vec::new();

        for e in edges {
            let s_in = member_set.contains(&e.source);
            let t_in = member_set.contains(&e.target);
            match (s_in, t_in) {
                (true, true) => internal.push(e.clone()),
                (false, true) => inbound.push(e.clone()),
                (true, false) => outbound.push(e.clone()),
                _ => {}
            }
        }
        Self {
            internal,
            inbound,
            outbound,
        }
    }
}

/// 计算静态 bool 条件下可从子图入口到达的成员节点。
fn compute_reachable_member_ids(
    member_nodes: &[BlueprintNode],
    classified: &ClassifiedEdges,
) -> HashSet<String> {
    let has_structural_incoming: HashSet<&str> = classified
        .internal
        .iter()
        .chain(classified.inbound.iter())
        .map(|edge| edge.target.as_str())
        .collect();

    let mut reachable: HashSet<String> = member_nodes
        .iter()
        .filter(|node| !has_structural_incoming.contains(node.id.as_str()))
        .map(|node| node.id.clone())
        .collect();
    reachable.extend(
        classified
            .inbound
            .iter()
            .filter(|edge| !edge_is_statically_disabled(edge))
            .map(|edge| edge.target.clone()),
    );

    loop {
        let mut changed = false;
        for edge in classified
            .internal
            .iter()
            .filter(|edge| !edge_is_statically_disabled(edge))
        {
            if reachable.contains(&edge.source) && reachable.insert(edge.target.clone()) {
                changed = true;
            }
        }
        if !changed {
            return reachable;
        }
    }
}

/// 将"无状态创建函数"包装为 Graph 需要的 NodeFactory。
///
/// 每次执行都生成全新的节点实例，保证节点状态隔离。
fn factory_from_creator<N, F>(creator: F) -> Arc<NodeFactory>
where
    N: AnyNode,
    F: Fn() -> N + Send + Sync + 'static,
{
    Arc::new(move || Ok(Arc::new(RwLock::new(creator())) as Arc<RwLock<dyn AnyNode>>))
}

/// 解析蓝图边的 condition 到运行时可执行条件。
///
/// 注意：为了保证语义保真，不支持的 condition 会直接返回错误，而不是忽略。
fn parse_edge_condition(condition: Option<Value>) -> Result<Option<EdgeCondition>, String> {
    match condition {
        None => Ok(None),
        Some(Value::Bool(b)) => Ok(Some(Box::new(move |_ctx: &Context, _out: &Output| b))),
        Some(other) => Err(format!("Unsupported edge condition: {}", other)),
    }
}

fn edge_is_statically_disabled(edge: &BlueprintEdge) -> bool {
    matches!(edge.condition, Some(Value::Bool(false)))
}

/// Reject edge semantics that the current runtime `Graph` cannot represent.
///
/// Control handles are intentionally accepted: they identify UI connection points but do not
/// change runtime routing. Data edges and port/type routing are different; compiling those into a
/// plain node-to-node `FlowEdge` would silently turn a port value into a control-flow value.
fn validate_runtime_edge_semantics(edges: &[BlueprintEdge]) -> Result<(), String> {
    let mut endpoint_pairs = HashSet::new();

    for edge in edges {
        if edge.kind == EdgeKind::Data {
            return Err(format!(
                "Edge '{}' is a data edge, but runtime Graph edges do not support port-aware data routing",
                edge.id
            ));
        }
        if edge.source_port.is_some() || edge.target_port.is_some() || edge.data_type.is_some() {
            return Err(format!(
                "Edge '{}' declares data port/type metadata, but runtime Graph edges cannot preserve it",
                edge.id
            ));
        }

        let pair = (edge.source.as_str(), edge.target.as_str());
        if !endpoint_pairs.insert(pair) {
            return Err(format!(
                "Parallel edges from '{}' to '{}' cannot be represented without losing per-edge semantics",
                edge.source, edge.target
            ));
        }
    }

    Ok(())
}

/// 从蓝图节点配置中解析子图执行配置。
///
/// # 参数
/// - `node`：子图容器节点，其 config 包含子图配置
/// - `entry_node`：子图入口节点 ID
/// - `exit_node`：子图出口节点 ID
fn parse_subgraph_config(
    node: &BlueprintNode,
    entry_node: String,
    exit_node: String,
) -> SubgraphConfig {
    SubgraphConfig {
        entry_node,
        exit_node,
        inherit_context: node
            .config
            .get("inherit_context")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        timeout: node
            .config
            .get("timeout_ms")
            .and_then(|v| v.as_u64())
            .map(std::time::Duration::from_millis),
    }
}

/// 子图构建器：负责将蓝图节点和边编译为可执行的子图。
///
/// 子图运行时需要统一的入口/出口节点，以及代理节点来处理跨边。
/// 构建器内部维护边的去重和节点连通性状态。
struct SubgraphBuilder<'a> {
    graph: Graph,
    group_id: &'a str,
    start_id: String,
    end_id: String,
    seen_edges: HashSet<(String, String)>,
    has_incoming: HashSet<String>,
}

impl<'a> SubgraphBuilder<'a> {
    /// 创建新的子图构建器。
    ///
    /// 自动创建统一的入口节点（`__subgraph_start__{group_id}`）
    /// 和出口节点（`__subgraph_end__{group_id}`）。
    fn new(group_id: &'a str) -> Self {
        let start_id = format!("__subgraph_start__{}", group_id);
        let end_id = format!("__subgraph_end__{}", group_id);

        let mut graph = Graph::new();
        graph.add_node(&start_id, || SubgraphStartNode);
        graph.add_node(&end_id, || SubgraphEndNode);

        Self {
            graph,
            group_id,
            start_id,
            end_id,
            seen_edges: HashSet::new(),
            has_incoming: HashSet::new(),
        }
    }

    /// 添加成员节点到子图。
    ///
    /// 普通节点由 `create_leaf_factory` 创建；
    /// 若成员节点本身也是子图容器，则复用已编译好的 `runtime_groups`。
    fn add_member_nodes<F, IsGroup>(
        &mut self,
        member_nodes: &[BlueprintNode],
        is_group: &IsGroup,
        create_leaf_factory: &F,
        runtime_groups: &HashMap<String, Arc<NodeFactory>>,
    ) -> Result<(), String>
    where
        F: Fn(&BlueprintNode) -> Result<Arc<NodeFactory>, String> + Clone,
        IsGroup: Fn(&BlueprintNode) -> bool,
    {
        for n in member_nodes {
            let factory = if is_group(n) {
                runtime_groups
                    .get(&n.id)
                    .cloned()
                    .ok_or_else(|| format!("Nested group '{}' has not been built yet", n.id))?
            } else {
                create_leaf_factory(n)?
            };
            self.graph.add_node_factory(&n.id, factory);
        }
        Ok(())
    }

    /// 设置入边代理节点。
    ///
    /// 子图运行时只从 start 接收统一入口，代理节点负责按 source id
    /// 从入口数据里取出对应值，再喂给内部目标节点。
    ///
    /// # 返回
    /// 外部 source ID 到代理节点 ID 的映射
    fn setup_inbound_proxies(
        &mut self,
        inbound_edges: &[BlueprintEdge],
    ) -> HashMap<String, String> {
        let mut source_to_proxy = HashMap::new();
        for e in inbound_edges {
            if source_to_proxy.contains_key(&e.source) {
                continue;
            }
            let proxy_id = format!("__subgraph_in__{}__{}", self.group_id, e.source);
            let key = e.source.clone();
            self.graph
                .add_node(&proxy_id, move || SubgraphInNode { key: key.clone() });
            self.add_edge(self.start_id.clone(), proxy_id.clone(), None);
            source_to_proxy.insert(e.source.clone(), proxy_id);
        }
        source_to_proxy
    }

    /// 分析边的连通性，统计每个节点是否有入边/出边。
    ///
    /// 用于后续判断哪些节点需要连接到 start/end。
    fn analyze_edge_connectivity(
        &mut self,
        internal_edges: &[BlueprintEdge],
        inbound_edges: &[BlueprintEdge],
    ) {
        for e in internal_edges {
            self.has_incoming.insert(e.target.clone());
        }
        for e in inbound_edges {
            self.has_incoming.insert(e.target.clone());
        }
    }

    /// 添加边（带去重）。
    ///
    /// 折叠/补边过程中可能产生重复的 (from, to)，需要去重避免重复执行。
    fn add_edge(&mut self, from: String, to: String, condition: Option<EdgeCondition>) {
        if self.seen_edges.insert((from.clone(), to.clone())) {
            self.graph.add_edge(FlowEdge {
                from,
                to,
                condition,
            });
        }
    }

    /// 构建子图内部边。
    fn build_internal_edges(&mut self, internal_edges: &[BlueprintEdge]) -> Result<(), String> {
        for e in internal_edges {
            if edge_is_statically_disabled(e) {
                continue;
            }
            let condition = parse_edge_condition(e.condition.clone())?;
            self.add_edge(e.source.clone(), e.target.clone(), condition);
        }
        Ok(())
    }

    /// 构建代理边：从代理节点连接到内部目标节点。
    ///
    /// 外部 -> 内部的跨边由代理节点转接，跨图边不在子图内部直接出现。
    fn build_proxy_edges(
        &mut self,
        inbound_edges: &[BlueprintEdge],
        source_to_proxy: &HashMap<String, String>,
    ) -> Result<(), String> {
        for e in inbound_edges {
            if edge_is_statically_disabled(e) {
                continue;
            }
            if let Some(proxy_id) = source_to_proxy.get(&e.source) {
                // The outer source -> group edge is only a transport edge. The original condition
                // belongs here so each inbound target keeps its own branch decision.
                let condition = parse_edge_condition(e.condition.clone())?;
                self.add_edge(proxy_id.clone(), e.target.clone(), condition);
            }
        }
        Ok(())
    }

    /// 移除从统一入口不可达的成员节点。
    ///
    /// 蓝图条件当前只有静态 bool。false 边不会进入运行时图；剪掉因此不可达的成员，
    /// 可以避免它们继续作为 AllUpstreamReady 的结构父节点阻塞有效分支和统一出口。
    fn prune_unreachable_members(&mut self, member_nodes: &[BlueprintNode]) {
        let mut reachable = HashSet::from([self.start_id.clone()]);
        let mut pending = vec![self.start_id.clone()];

        while let Some(node_id) = pending.pop() {
            if let Some(edges) = self.graph.edges.get(&node_id) {
                for edge in edges {
                    let target = edge.to().to_owned();
                    if reachable.insert(target.clone()) {
                        pending.push(target);
                    }
                }
            }
        }

        for node in member_nodes {
            if !reachable.contains(&node.id) {
                self.graph.remove_node(&node.id);
            }
        }
    }

    /// 连接入口节点：没有任何入边的节点由 start 直接触发。
    ///
    /// 避免子图启动后无可执行节点。
    fn connect_entry_nodes(&mut self, member_nodes: &[BlueprintNode]) {
        for n in member_nodes {
            if !self.has_incoming.contains(&n.id) {
                self.add_edge(self.start_id.clone(), n.id.clone(), None);
            }
        }
    }

    /// 连接出口节点：将出口候选统一连到 end。
    ///
    /// 出口选择规则：
    /// - 若存在内部 -> 外部跨边，则这些跨边的 source 作为出口候选
    /// - 否则，选择内部图里"无出边"的末端节点作为出口候选
    /// - 若没有候选（例如空子图），则 start -> end
    fn connect_exit_nodes(
        &mut self,
        member_nodes: &[BlueprintNode],
        outbound_edges: &[BlueprintEdge],
    ) {
        let enabled_outbound_sources: HashSet<String> = outbound_edges
            .iter()
            .filter(|edge| !edge_is_statically_disabled(edge))
            .filter(|edge| self.graph.nodes.contains_key(&edge.source))
            .map(|edge| edge.source.clone())
            .collect();

        let exit_sources: HashSet<String> = if !enabled_outbound_sources.is_empty() {
            enabled_outbound_sources
        } else {
            member_nodes
                .iter()
                .filter(|node| self.graph.nodes.contains_key(&node.id))
                .filter(|node| {
                    self.graph
                        .edges
                        .get(&node.id)
                        .is_none_or(|edges| edges.is_empty())
                })
                .map(|n| n.id.clone())
                .collect()
        };

        if exit_sources.is_empty() {
            self.add_edge(self.start_id.clone(), self.end_id.clone(), None);
        } else {
            for src in exit_sources {
                self.add_edge(src, self.end_id.clone(), None);
            }
        }
    }

    /// 构建子图执行器。
    ///
    /// 按顺序执行：添加成员节点 -> 设置代理 -> 分析连通性 -> 构建边 -> 连接入出口
    fn build<F, IsGroup>(
        mut self,
        group_node: &BlueprintNode,
        member_nodes: &[BlueprintNode],
        classified: ClassifiedEdges,
        is_group: &IsGroup,
        create_leaf_factory: &F,
        runtime_groups: &HashMap<String, Arc<NodeFactory>>,
    ) -> Result<SubgraphExecutor, String>
    where
        F: Fn(&BlueprintNode) -> Result<Arc<NodeFactory>, String> + Clone,
        IsGroup: Fn(&BlueprintNode) -> bool,
    {
        self.add_member_nodes(member_nodes, is_group, create_leaf_factory, runtime_groups)?;

        let source_to_proxy = self.setup_inbound_proxies(&classified.inbound);

        self.analyze_edge_connectivity(&classified.internal, &classified.inbound);
        self.build_internal_edges(&classified.internal)?;
        self.build_proxy_edges(&classified.inbound, &source_to_proxy)?;
        self.connect_entry_nodes(member_nodes);
        self.prune_unreachable_members(member_nodes);
        self.connect_exit_nodes(member_nodes, &classified.outbound);

        let config = parse_subgraph_config(group_node, self.start_id, self.end_id);
        Ok(SubgraphExecutor::new(self.graph, config))
    }
}

/// 计算子图的嵌套深度。
///
/// 用于确定编译顺序：先编译最深层，再逐层向外折叠。
fn compute_group_depth(group_id: &str, group_parent: &HashMap<String, String>) -> usize {
    let mut cur = group_id.to_string();
    let mut depth = 0;
    let mut visited = HashSet::new();
    while let Some(p) = group_parent.get(&cur) {
        if !visited.insert(cur.clone()) {
            break;
        }
        depth += 1;
        cur = p.clone();
    }
    depth
}

/// 折叠所有子图分组。
///
/// 将蓝图中的分组节点编译为子图执行器，并折叠成员节点和跨边。
///
/// # 返回
/// - 折叠后的节点列表（成员节点被移除，保留容器节点）
/// - 折叠后的边列表（跨边被改写为连接到容器节点）
/// - 运行时子图工厂映射（group_id -> NodeFactory）
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
    let group_ids: Vec<String> = nodes
        .iter()
        .filter(|n| is_group(n))
        .map(|n| n.id.clone())
        .collect();

    if group_ids.is_empty() {
        return Ok((nodes.to_vec(), edges.to_vec(), HashMap::new()));
    }

    let group_id_set: HashSet<String> = group_ids.iter().cloned().collect();
    let group_parent: HashMap<String, String> = nodes
        .iter()
        .filter(|n| is_group(n))
        .filter_map(|n| {
            n.parent_id
                .as_ref()
                .filter(|pid| group_id_set.contains(*pid))
                .map(|pid| (n.id.clone(), pid.clone()))
        })
        .collect();

    let mut sorted_group_ids = group_ids;
    sorted_group_ids.sort_by_key(|id| std::cmp::Reverse(compute_group_depth(id, &group_parent)));

    let mut nodes = nodes.to_vec();
    let mut edges = edges.to_vec();
    let mut runtime_groups: HashMap<String, Arc<NodeFactory>> = HashMap::new();

    for group_id in sorted_group_ids {
        let group_node = match nodes.iter().find(|n| n.id == group_id) {
            Some(n) => n.clone(),
            None => continue,
        };

        let member_set: HashSet<String> = nodes
            .iter()
            .filter(|n| n.parent_id.as_deref() == Some(group_node.id.as_str()))
            .map(|n| n.id.clone())
            .collect();

        let member_nodes: Vec<BlueprintNode> = nodes
            .iter()
            .filter(|n| member_set.contains(&n.id))
            .cloned()
            .collect();

        let classified = ClassifiedEdges::classify(&edges, &member_set);
        let reachable_members = compute_reachable_member_ids(&member_nodes, &classified);

        let outbound_sources: HashSet<&str> = classified
            .outbound
            .iter()
            .filter(|edge| !edge_is_statically_disabled(edge))
            .filter(|edge| reachable_members.contains(&edge.source))
            .map(|edge| edge.source.as_str())
            .collect();
        if outbound_sources.len() > 1 {
            let mut sources: Vec<_> = outbound_sources.into_iter().collect();
            sources.sort_unstable();
            return Err(format!(
                "Subgraph '{}' has outbound edges from multiple internal sources ({}); its single output cannot preserve source-specific values and conditions",
                group_id,
                sources.join(", ")
            ));
        }

        let executor = SubgraphBuilder::new(&group_id).build(
            &group_node,
            &member_nodes,
            classified.clone(),
            is_group,
            create_leaf_factory,
            &runtime_groups,
        )?;

        let group_factory = create_group_factory(&group_node, executor)?;
        runtime_groups.insert(group_node.id.clone(), group_factory);

        nodes.retain(|n| !member_set.contains(&n.id));
        edges.retain(|e| !(member_set.contains(&e.source) || member_set.contains(&e.target)));

        let mut seen: HashSet<(String, String)> = edges
            .iter()
            .map(|e| (e.source.clone(), e.target.clone()))
            .collect();
        let preexisting_pairs = seen.clone();
        let enabled_inbound_sources: HashSet<String> = classified
            .inbound
            .iter()
            .filter(|edge| !edge_is_statically_disabled(edge))
            .map(|edge| edge.source.clone())
            .collect();

        for e in classified.inbound {
            let key = (e.source.clone(), group_node.id.clone());
            if preexisting_pairs.contains(&key) {
                return Err(format!(
                    "Subgraph '{}' has both a direct edge and an inbound child edge from '{}'; folding cannot preserve both routes",
                    group_node.id, e.source
                ));
            }
            if seen.insert(key) {
                let transport_condition = if enabled_inbound_sources.contains(&e.source) {
                    None
                } else {
                    Some(Value::Bool(false))
                };
                edges.push(BlueprintEdge {
                    id: format!("fold:{}:in:{}", group_node.id, e.id),
                    source: e.source,
                    target: group_node.id.clone(),
                    source_handle: e.source_handle,
                    target_handle: None,
                    // Conditions are evaluated by the proxy -> target edge inside the subgraph.
                    // This transport edge must stay unconditional so one false branch does not
                    // suppress every other inbound branch from the same source.
                    condition: transport_condition,
                    kind: EdgeKind::Control,
                    source_port: None,
                    target_port: None,
                    data_type: None,
                });
            }
        }

        for e in classified.outbound {
            let key = (group_node.id.clone(), e.target.clone());
            if preexisting_pairs.contains(&key) {
                return Err(format!(
                    "Subgraph '{}' has both a direct edge and an outbound child edge to '{}'; folding cannot preserve both routes",
                    group_node.id, e.target
                ));
            }
            if seen.insert(key) {
                let transport_condition = if reachable_members.contains(&e.source) {
                    e.condition
                } else {
                    Some(Value::Bool(false))
                };
                edges.push(BlueprintEdge {
                    id: format!("fold:{}:out:{}", group_node.id, e.id),
                    source: group_node.id.clone(),
                    target: e.target,
                    source_handle: None,
                    target_handle: e.target_handle,
                    condition: transport_condition,
                    kind: EdgeKind::Control,
                    source_port: None,
                    target_port: None,
                    data_type: None,
                });
            }
        }
    }

    Ok((nodes, edges, runtime_groups))
}

/// 编译蓝图为可执行 `Graph`（支持自定义分组判断和工厂创建）。
///
/// # 参数
/// - `nodes`：蓝图节点列表
/// - `edges`：蓝图边列表
/// - `is_group`：判断节点是否为分组容器的函数
/// - `create_group_factory`：创建分组节点工厂的函数
/// - `create_leaf_factory`：创建叶子节点工厂的函数
///
/// # 返回
/// - `(Arc<Graph>, start_nodes)`：可执行图和起始节点列表
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
    validate_runtime_edge_semantics(edges)?;

    let (nodes, edges, runtime_groups) = fold_groups(
        nodes,
        edges,
        &is_group,
        &create_group_factory,
        &create_leaf_factory,
    )?;

    let mut graph = Graph::new();
    let node_id_set: HashSet<&str> = nodes.iter().map(|n| n.id.as_str()).collect();

    let valid_edges: Vec<&BlueprintEdge> = edges
        .iter()
        .filter(|e| {
            node_id_set.contains(e.source.as_str()) && node_id_set.contains(e.target.as_str())
        })
        .collect();

    let has_incoming: HashSet<&str> = valid_edges.iter().map(|e| e.target.as_str()).collect();

    let mut start_nodes = Vec::new();
    for node in &nodes {
        let factory = if is_group(node) {
            runtime_groups
                .get(&node.id)
                .cloned()
                .ok_or_else(|| format!("Group '{}' not built", node.id))?
        } else {
            create_leaf_factory(node)?
        };
        graph.add_node_factory(&node.id, factory);
        if !has_incoming.contains(node.id.as_str()) {
            start_nodes.push(node.id.clone());
        }
    }

    for edge in &valid_edges {
        let condition = parse_edge_condition(edge.condition.clone())?;
        graph.add_edge(FlowEdge {
            from: edge.source.clone(),
            to: edge.target.clone(),
            condition,
        });
    }

    Ok((Arc::new(graph), start_nodes))
}

/// 编译蓝图为可执行 `Graph`。
///
/// 简化版本，通过 `group_type` 字符串判断分组节点。
///
/// # 参数
/// - `nodes`：蓝图节点列表
/// - `edges`：蓝图边列表
/// - `group_type`：分组容器节点的 type_name
/// - `create_leaf_factory`：创建叶子节点工厂的函数
///
/// # 返回
/// - `(Arc<Graph>, start_nodes)`：可执行图和起始节点列表（入度为 0 的节点）
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn unused_factory() -> Arc<NodeFactory> {
        Arc::new(|| Err("test factory should not be instantiated".to_string()))
    }

    fn node(id: &str, parent_id: Option<&str>) -> BlueprintNode {
        BlueprintNode {
            id: id.to_string(),
            type_name: if id == "G" { "Group" } else { "Leaf" }.to_string(),
            parent_id: parent_id.map(str::to_string),
            config: Value::Null,
        }
    }

    fn control_edge(
        id: &str,
        source: &str,
        target: &str,
        condition: Option<bool>,
    ) -> BlueprintEdge {
        BlueprintEdge {
            id: id.to_string(),
            source: source.to_string(),
            target: target.to_string(),
            source_handle: None,
            target_handle: None,
            condition: condition.map(Value::Bool),
            kind: EdgeKind::Control,
            source_port: None,
            target_port: None,
            data_type: None,
        }
    }

    #[test]
    fn disabled_inbound_branches_are_removed_after_the_source_proxy() {
        let inbound = vec![
            control_edge("e-sx", "S", "X", Some(false)),
            control_edge("e-sy", "S", "Y", Some(true)),
        ];
        let mut builder = SubgraphBuilder::new("G");
        let proxies = builder.setup_inbound_proxies(&inbound);

        builder
            .build_proxy_edges(&inbound, &proxies)
            .expect("valid boolean conditions should compile");

        let proxy_id = proxies.get("S").expect("source proxy should exist");
        let outgoing = builder
            .graph
            .edges
            .get(proxy_id)
            .expect("proxy should have outgoing edges");
        let output = Output::new(Some(json!({"value": 1})));
        let context = Context::new();

        assert!(outgoing.iter().all(|edge| edge.to() != "X"));
        assert!(
            outgoing
                .iter()
                .find(|edge| edge.to() == "Y")
                .expect("Y edge should exist")
                .check_condition(&context, &output)
        );
    }

    #[test]
    fn folded_inbound_transport_edge_is_single_and_unconditional() {
        let nodes = vec![
            node("S", None),
            node("G", None),
            node("X", Some("G")),
            node("Y", Some("G")),
        ];
        let edges = vec![
            control_edge("e-sx", "S", "X", Some(false)),
            control_edge("e-sy", "S", "Y", Some(true)),
        ];

        let (_, folded_edges, _) = fold_groups(
            &nodes,
            &edges,
            &|node| node.type_name == "Group",
            &|_, _| Ok(unused_factory()),
            &|_| Ok(unused_factory()),
        )
        .expect("control edges should fold");

        let transport_edges: Vec<_> = folded_edges
            .iter()
            .filter(|edge| edge.source == "S" && edge.target == "G")
            .collect();
        assert_eq!(transport_edges.len(), 1);
        assert_eq!(transport_edges[0].condition, None);
    }

    #[test]
    fn folded_inbound_transport_is_disabled_when_every_branch_is_false() {
        let nodes = vec![node("S", None), node("G", None), node("X", Some("G"))];
        let edges = vec![control_edge("e-sx", "S", "X", Some(false))];

        let (_, folded_edges, _) = fold_groups(
            &nodes,
            &edges,
            &|node| node.type_name == "Group",
            &|_, _| Ok(unused_factory()),
            &|_| Ok(unused_factory()),
        )
        .expect("static false control edge should fold");

        let transport = folded_edges
            .iter()
            .find(|edge| edge.source == "S" && edge.target == "G")
            .expect("folding should retain a disabled structural edge");
        assert_eq!(transport.condition, Some(Value::Bool(false)));
    }

    #[test]
    fn data_edges_fail_instead_of_silently_becoming_control_edges() {
        let nodes = vec![node("A", None), node("B", None)];
        let mut edge = control_edge("data-edge", "A", "B", None);
        edge.kind = EdgeKind::Data;
        edge.source_port = Some("result".to_string());
        edge.target_port = Some("input".to_string());
        edge.data_type = Some(DataType::Json);

        let error = match compile_graph(&nodes, &[edge], "Group", |_| Ok(unused_factory())) {
            Ok(_) => panic!("the runtime graph has no port-aware data edge representation"),
            Err(error) => error,
        };

        assert!(error.contains("data-edge"));
        assert!(error.contains("data edge"));
    }

    #[test]
    fn port_metadata_on_control_edges_is_not_silently_ignored() {
        let nodes = vec![node("A", None), node("B", None)];
        let mut edge = control_edge("ported-control-edge", "A", "B", None);
        edge.source_port = Some("result".to_string());

        let error = match compile_graph(&nodes, &[edge], "Group", |_| Ok(unused_factory())) {
            Ok(_) => panic!("port metadata requires a port-aware runtime edge"),
            Err(error) => error,
        };

        assert!(error.contains("ported-control-edge"));
        assert!(error.contains("port/type metadata"));
    }

    #[test]
    fn control_handles_remain_accepted_as_ui_metadata() {
        let nodes = vec![node("A", None), node("B", None)];
        let mut edge = control_edge("control-edge", "A", "B", None);
        edge.source_handle = Some("ctrl".to_string());
        edge.target_handle = Some("ctrl".to_string());

        compile_graph(&nodes, &[edge], "Group", |_| Ok(unused_factory()))
            .expect("control handles do not affect runtime routing");
    }

    #[test]
    fn parallel_control_edges_fail_instead_of_losing_one_condition() {
        let nodes = vec![node("A", None), node("B", None)];
        let edges = vec![
            control_edge("e-true", "A", "B", Some(true)),
            control_edge("e-false", "A", "B", Some(false)),
        ];

        let error = match compile_graph(&nodes, &edges, "Group", |_| Ok(unused_factory())) {
            Ok(_) => panic!("parallel endpoint semantics require edge identity at runtime"),
            Err(error) => error,
        };

        assert!(error.contains("Parallel edges"));
        assert!(error.contains("A"));
        assert!(error.contains("B"));
    }

    #[test]
    fn direct_group_and_inbound_child_edges_cannot_compete_for_one_route() {
        let nodes = vec![node("S", None), node("G", None), node("X", Some("G"))];
        let edges = vec![
            control_edge("e-direct", "S", "G", None),
            control_edge("e-child", "S", "X", None),
        ];

        let error = match compile_graph(&nodes, &edges, "Group", |_| Ok(unused_factory())) {
            Ok(_) => panic!("folding would collapse direct and child routes"),
            Err(error) => error,
        };

        assert!(error.contains("direct edge"));
        assert!(error.contains("inbound child edge"));
    }

    #[test]
    fn multiple_outbound_sources_are_rejected_when_their_values_cannot_be_routed() {
        let nodes = vec![
            node("G", None),
            node("X", Some("G")),
            node("Y", Some("G")),
            node("T", None),
        ];
        let edges = vec![
            control_edge("e-xt", "X", "T", None),
            control_edge("e-yt", "Y", "T", None),
        ];

        let error = match compile_graph(&nodes, &edges, "Group", |_| Ok(unused_factory())) {
            Ok(_) => panic!("a single group output cannot preserve both source values"),
            Err(error) => error,
        };

        assert!(error.contains("G"));
        assert!(error.contains("outbound"));
    }

    #[test]
    fn outbound_from_unreachable_member_stays_structurally_disabled() {
        let nodes = vec![
            node("G", None),
            node("A", Some("G")),
            node("X", Some("G")),
            node("T", None),
        ];
        let edges = vec![
            control_edge("e-ax", "A", "X", Some(false)),
            control_edge("e-xt", "X", "T", None),
        ];

        let (_, folded_edges, _) = fold_groups(
            &nodes,
            &edges,
            &|node| node.type_name == "Group",
            &|_, _| Ok(unused_factory()),
            &|_| Ok(unused_factory()),
        )
        .expect("statically unreachable outbound source should fold safely");

        let outbound = folded_edges
            .iter()
            .find(|edge| edge.source == "G" && edge.target == "T")
            .expect("disabled edge keeps T from becoming an unrelated start node");
        assert_eq!(outbound.condition, Some(Value::Bool(false)));
    }
}
