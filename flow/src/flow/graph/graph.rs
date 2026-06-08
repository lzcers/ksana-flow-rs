use async_trait::async_trait;

use super::io::{Input, Output};
use crate::Context;
use std::{any::Any, collections::HashMap, sync::Arc};
use tokio::sync::RwLock;

pub type NodeId = String;

// 核心节点定义
#[async_trait]
pub trait Node {
    // 节点的调度策略，这决定了节点何时被触发执行
    // 默认是全量上游就绪才触发
    // 理论上应该是调度器的配置而不是节点的设置，由用户来决定该节点该如何调度
    // 同一个节点在不同的场景下可能有不同的调度策略，例如在条件分支中
    const TRIGGER_STRATEGY: TriggerStrategy = TriggerStrategy::AllUpstreamReady;

    async fn run(&mut self, ctx: &Context, input: &Input) -> Result<Output, String>;
}

pub type EdgeCondition = Box<dyn Fn(&Context, &Output) -> bool + Send + Sync>;

// 条件边接收一个节点传出的输出引用，根据条件判断是否继续执行下一个节点
pub struct Edge {
    pub from: String,
    pub to: String,
    pub condition: Option<EdgeCondition>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TriggerStrategy {
    /// 全量上游就绪才触发
    AllUpstreamReady,
    /// 任意上游有输入即触发
    AnyUpstreamAvailable,
}
impl Default for TriggerStrategy {
    fn default() -> Self {
        TriggerStrategy::AllUpstreamReady
    }
}

// AnyNode 会被Arc 容器包裹发送到不同的线程中执行，因此需要实现 Send + Sync
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

// 子图情况下，节点和边都需要被 Arc 包裹，因为会被发送到不同的线程中执行
pub type NodeFactory = dyn Fn() -> Result<Arc<RwLock<dyn AnyNode>>, String> + Send + Sync;

pub struct Graph {
    pub nodes: HashMap<NodeId, Arc<NodeFactory>>,
    pub edges: HashMap<NodeId, Vec<Arc<dyn AnyEdge>>>,
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
