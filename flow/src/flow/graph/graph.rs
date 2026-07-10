use async_trait::async_trait;

use super::io::{Input, Output};
use crate::Context;
use std::{any::Any, collections::HashMap, sync::Arc};
use tokio::sync::RwLock;

/// 一张图内节点的唯一标识，同时也是下游 `Input` 中默认的来源键。
pub type NodeId = String;

/// 用户实现的核心节点接口。
///
/// Runner 会为每次执行物化一组节点实例。`run` 接收共享的运行时 `Context`
/// 和由上游节点 ID 索引的 `Input`；节点内部的可变状态只属于当前 Runner。
#[async_trait]
pub trait Node {
    // 节点的调度策略，这决定了节点何时被触发执行
    // 默认是全量上游就绪才触发
    // 理论上应该是调度器的配置而不是节点的设置，由用户来决定该节点该如何调度
    // 同一个节点在不同的场景下可能有不同的调度策略，例如在条件分支中
    const TRIGGER_STRATEGY: TriggerStrategy = TriggerStrategy::AllUpstreamReady;

    /// 执行一次节点逻辑。
    ///
    /// 返回 `Output` 后，Runner 才会更新节点状态并尝试调度下游。
    async fn run(&mut self, ctx: &Context, input: &Input) -> Result<Output, String>;
}

/// 运行时边条件。条件只读取当前来源节点的输出和共享上下文。
pub type EdgeCondition = Box<dyn Fn(&Context, &Output) -> bool + Send + Sync>;

/// 有向运行时边；可选条件决定当前输出是否沿该边传播。
pub struct Edge {
    pub from: String,
    pub to: String,
    pub condition: Option<EdgeCondition>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TriggerStrategy {
    /// 所有结构父节点均为 `Completed` 或 `Skipped` 后触发，并聚合其输出。
    AllUpstreamReady,
    /// 任意一条通过条件的上游边产生输出时触发，只携带该来源的输出。
    AnyUpstreamAvailable,
}
impl Default for TriggerStrategy {
    fn default() -> Self {
        TriggerStrategy::AllUpstreamReady
    }
}

/// `Node` 的对象安全适配层，供 Graph 保存异构节点。
///
/// 实例会被 `Arc<RwLock<_>>` 包装；写锁保证同一节点实例的 `run` 不会并发执行。
#[async_trait]
pub trait AnyNode: Any + Send + Sync {
    fn get_trigger_strategy(&self) -> TriggerStrategy {
        TriggerStrategy::AllUpstreamReady
    }

    async fn run(&mut self, ctx: &Context, input: &Input) -> Result<Output, String>;
}

#[async_trait]
impl<N: Node + Any + Send + Sync> AnyNode for N {
    fn get_trigger_strategy(&self) -> TriggerStrategy {
        N::TRIGGER_STRATEGY
    }
    async fn run(&mut self, ctx: &Context, input: &Input) -> Result<Output, String> {
        <N as Node>::run(self, ctx, input).await
    }
}

pub trait AnyEdge: Send + Sync {
    fn from(&self) -> &str;
    fn to(&self) -> &str;
    fn check_condition(&self, ctx: &Context, output: &Output) -> bool;
}

impl AnyEdge for Edge {
    fn from(&self) -> &str {
        &self.from
    }

    fn to(&self) -> &str {
        &self.to
    }

    fn check_condition(&self, ctx: &Context, output: &Output) -> bool {
        match &self.condition {
            Some(condition) => condition(ctx, output),
            None => true,
        }
    }
}

/// 节点实例工厂。每个 Runner 物化图时都会调用一次，以隔离节点内部状态。
pub type NodeFactory = dyn Fn() -> Result<Arc<RwLock<dyn AnyNode>>, String> + Send + Sync;

/// 可执行图及其两张邻接索引。
///
/// `edges` 与 `incoming_nodes` 必须描述同一组边。修改图时应优先使用
/// `add_edge`、`remove_edge` 和 `remove_node`，避免破坏该不变量。
pub struct Graph {
    /// 节点 ID 到运行时工厂的映射。
    pub nodes: HashMap<NodeId, Arc<NodeFactory>>,
    /// 按来源节点索引的出边。
    pub edges: HashMap<NodeId, Vec<Arc<dyn AnyEdge>>>,
    /// 按目标节点索引的父节点 ID，用于就绪判断和输入聚合。
    pub incoming_nodes: HashMap<NodeId, Vec<NodeId>>,
}

// 手动实现 Clone，因为 Box<dyn AnyEdge> 不能自动 Clone
impl Clone for Graph {
    fn clone(&self) -> Self {
        Self {
            nodes: self.nodes.clone(),
            edges: self.edges.clone(),
            incoming_nodes: self.incoming_nodes.clone(),
        }
    }
}

impl Graph {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            edges: HashMap::new(),
            incoming_nodes: HashMap::new(),
        }
    }
    pub fn get_node_ids(&self) -> Vec<String> {
        self.nodes.keys().cloned().collect()
    }

    pub fn get_parents(&self, node_id: &str) -> Vec<String> {
        self.incoming_nodes
            .get(node_id)
            .cloned()
            .unwrap_or_default()
    }

    pub fn add_node<N, F>(&mut self, id: &str, creator: F)
    where
        N: AnyNode,
        F: Fn() -> N + Send + Sync + 'static,
    {
        self.nodes.insert(
            id.to_owned(),
            Arc::new(move || Ok(Arc::new(RwLock::new(creator())) as Arc<RwLock<dyn AnyNode>>)),
        );
    }

    pub fn add_edge(&mut self, edge: Edge) {
        self.incoming_nodes
            .entry(edge.to.clone())
            .or_insert_with(Vec::new)
            .push(edge.from.clone());

        self.edges
            .entry(edge.from.clone())
            .or_insert_with(Vec::new)
            .push(Arc::new(edge));
    }

    pub fn add_node_factory(&mut self, id: &str, factory: Arc<NodeFactory>) {
        self.nodes.insert(id.to_owned(), factory);
    }

    pub fn remove_node(&mut self, id: &str) {
        self.nodes.remove(id);

        // Remove outgoing edges and update incoming_nodes of children
        if let Some(outgoing_edges) = self.edges.remove(id) {
            for edge in outgoing_edges {
                if let Some(parents) = self.incoming_nodes.get_mut(edge.to()) {
                    parents.retain(|p| p != id);
                }
            }
        }

        if let Some(parents) = self.incoming_nodes.remove(id) {
            for parent_id in parents {
                if let Some(edges) = self.edges.get_mut(&parent_id) {
                    edges.retain(|e| e.to() != id);
                }
            }
        }
    }

    pub fn remove_edge(&mut self, from: &str, to: &str) {
        if let Some(edges) = self.edges.get_mut(from) {
            edges.retain(|e| e.to() != to);
        }

        if let Some(parents) = self.incoming_nodes.get_mut(to) {
            parents.retain(|p| p != from);
        }
    }
}
