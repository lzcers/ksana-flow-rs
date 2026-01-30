use async_trait::async_trait;
use dashmap::DashMap;
use serde_json::Value;
use std::{any::Any, collections::HashMap, sync::Arc};
use tokio::sync::RwLock;

use super::io::{Input, Output};

pub type NodeId = String;

#[async_trait]
pub trait Node {
    const TRIGGER_STRATEGY: TriggerStrategy = TriggerStrategy::AllUpstreamReady;

    async fn run(&mut self, ctx: &Context, input: &Input) -> Result<Output, String>;
}

pub type EdgeCondition = Box<dyn Fn(&Context, &Output) -> bool + Send>;

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
// 节点运行上下文
// Context 内部是一个并发安全的结构，因此 Context 只要能 Clone 就行
#[derive(Clone, Debug)]
pub struct Context {
    data: DashMap<String, Value>,
}

impl Context {
    pub fn new() -> Self {
        Self {
            data: DashMap::new(),
        }
    }

    pub fn set(&self, key: impl Into<String>, value: impl serde::Serialize) {
        let value = serde_json::to_value(value).expect("Failed to serialize value");
        self.data.insert(key.into(), value);
    }

    pub fn get<T: serde::de::DeserializeOwned>(&self, key: &str) -> Option<T> {
        self.data
            .get(key)
            .and_then(|v| serde_json::from_value(v.clone()).ok())
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

pub trait AnyEdge: Send {
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

pub struct Graph {
    pub nodes: HashMap<NodeId, Arc<RwLock<dyn AnyNode>>>,
    pub edges: HashMap<NodeId, Vec<Box<dyn AnyEdge>>>,
    pub incoming_nodes: HashMap<NodeId, Vec<NodeId>>,
    pub node_trigger_strategy: HashMap<NodeId, TriggerStrategy>,
}

impl Graph {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            edges: HashMap::new(),
            incoming_nodes: HashMap::new(),
            node_trigger_strategy: HashMap::new(),
        }
    }
    pub fn get_node_ids(&self) -> Vec<String> {
        self.nodes.keys().cloned().collect()
    }

    pub fn get_trigger_strategy(&self, node_id: &str) -> TriggerStrategy {
        self.node_trigger_strategy
            .get(node_id)
            .cloned()
            .unwrap_or_default()
    }

    pub fn get_parents(&self, node_id: &str) -> Vec<String> {
        self.incoming_nodes
            .get(node_id)
            .cloned()
            .unwrap_or_default()
    }

    pub fn add_node<N: AnyNode>(&mut self, id: &str, node: N) {
        let node_trigger_strategy = node.get_trigger_strategy();
        self.nodes.insert(
            id.to_owned(),
            Arc::new(RwLock::new(node)) as Arc<RwLock<dyn AnyNode>>,
        );
        self.node_trigger_strategy
            .insert(id.to_owned(), node_trigger_strategy);
    }

    pub fn add_edge(&mut self, edge: Edge) {
        self.incoming_nodes
            .entry(edge.to.clone())
            .or_insert_with(Vec::new)
            .push(edge.from.clone());

        self.edges
            .entry(edge.from.clone())
            .or_insert_with(Vec::new)
            .push(Box::new(edge));
    }

    pub fn add_arc_node(
        &mut self,
        id: &str,
        node: Arc<RwLock<dyn AnyNode>>,
        trigger_strategy: Option<TriggerStrategy>,
    ) {
        self.nodes.insert(id.to_owned(), node);
        self.node_trigger_strategy
            .insert(id.to_owned(), trigger_strategy.unwrap_or_default());
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
