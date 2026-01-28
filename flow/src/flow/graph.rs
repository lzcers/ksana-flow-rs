use async_trait::async_trait;
use dashmap::DashMap;
use serde_json::Value;
use std::{any::Any, collections::HashMap, sync::Arc};
use tokio::sync::RwLock;

use super::output_payload::OutputPayload;

pub type NodeId = String;

#[derive(Clone)]
pub struct NodeInputs {
    pub inputs: HashMap<NodeId, OutputPayload>,
}

impl NodeInputs {
    pub fn new(inputs: HashMap<NodeId, OutputPayload>) -> Self {
        Self { inputs }
    }

    pub fn get<T: 'static>(&self, key: &str) -> Option<&T> {
        self.inputs
            .get(key)
            .and_then(|p| p.as_any())
            .and_then(|any| any.downcast_ref::<T>())
    }

    pub fn get_any(&self) -> Option<&OutputPayload> {
        self.inputs.values().next()
    }

    pub fn get_payload(&self, key: &str) -> Option<&OutputPayload> {
        self.inputs.get(key)
    }

    pub fn iter_any(&self) -> impl Iterator<Item = (&NodeId, &dyn Any)> + '_ {
        self.inputs.iter().filter_map(|(k, v)| {
            let any = v.as_any()?;
            Some((k, any))
        })
    }
}

#[async_trait]
pub trait Node {
    const TRIGGER_STRATEGY: TriggerStrategy = TriggerStrategy::AllUpstreamReady;

    async fn run(&mut self, ctx: &Context, inputs: NodeInputs) -> OutputPayload;
}

pub type EdgeCondition<Out> = Box<dyn Fn(&Context, &Out) -> bool + Send>;

// 条件边接收一个节点传出的输出引用，根据条件判断是否继续执行下一个节点
pub struct Edge<Out = ()> {
    pub from: String,
    pub to: String,
    pub condition: Option<EdgeCondition<Out>>,
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
    fn as_any(&self) -> &dyn Any;

    fn get_trigger_strategy(&self) -> TriggerStrategy {
        TriggerStrategy::AllUpstreamReady
    }
    fn as_any_mut(&mut self) -> &mut dyn Any;
    async fn run(&mut self, ctx: &Context, inputs: NodeInputs) -> Result<OutputPayload, String>;
}

#[async_trait]
impl<N: Node + Any + Send + Sync> AnyNode for N {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn get_trigger_strategy(&self) -> TriggerStrategy {
        N::TRIGGER_STRATEGY
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
    async fn run(&mut self, ctx: &Context, inputs: NodeInputs) -> Result<OutputPayload, String> {
        Ok(self.run(ctx, inputs).await)
    }
}

pub trait AnyEdge: Any + Send {
    fn as_any(&self) -> &dyn Any;
    fn from(&self) -> &str;
    fn to(&self) -> &str;
    fn check_condition(&self, ctx: &Context, output: &dyn Any) -> bool;
}

impl<Out: Any> AnyEdge for Edge<Out> {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn from(&self) -> &str {
        &self.from
    }

    fn to(&self) -> &str {
        &self.to
    }

    fn check_condition(&self, ctx: &Context, output: &dyn Any) -> bool {
        let output = (output as &dyn Any).downcast_ref::<Out>();
        match (&self.condition, output) {
            (Some(condition), Some(out)) => condition(ctx, out),
            (None, _) => true,
            _ => false,
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

    pub fn add_edge<Out: Any>(&mut self, edge: Edge<Out>) {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct StrategyNode;

    #[async_trait]
    impl Node for StrategyNode {
        const TRIGGER_STRATEGY: TriggerStrategy = TriggerStrategy::AnyUpstreamAvailable;

        async fn run(&mut self, _ctx: &Context, _inputs: NodeInputs) -> OutputPayload {
            OutputPayload::shared(())
        }
    }

    #[test]
    fn test_graph_add_node_reads_strategy_from_node() {
        let mut graph = Graph::new();
        graph.add_node("n", StrategyNode::default());
        assert_eq!(
            graph.get_trigger_strategy("n"),
            TriggerStrategy::AnyUpstreamAvailable
        );
    }

    #[test]
    fn test_anynode_trait_object_can_read_strategy() {
        let node: Box<dyn AnyNode> = Box::new(StrategyNode::default());
        assert_eq!(
            node.get_trigger_strategy(),
            TriggerStrategy::AnyUpstreamAvailable
        );
    }

    #[test]
    fn test_edge_type_mismatch_unconditional() {
        // Create an edge expecting i32
        let edge: Edge<i32> = Edge {
            from: "a".to_string(),
            to: "b".to_string(),
            condition: None,
        };
        let ctx = Context::new();

        // Pass a string
        let output = "hello".to_string();
        let any_output: &dyn Any = &output;

        // Check condition
        assert!(edge.check_condition(&ctx, any_output));
    }

    #[test]
    fn test_edge_type_mismatch_conditional() {
        // Create an edge expecting i32 with condition
        let edge: Edge<i32> = Edge {
            from: "a".to_string(),
            to: "b".to_string(),
            condition: Some(Box::new(|_, val| *val > 0)),
        };
        let ctx = Context::new();

        // Pass a string
        let output = "hello".to_string();
        let any_output: &dyn Any = &output;

        // Check condition - should be false because type mismatch makes output None, matching _ => false
        assert!(!edge.check_condition(&ctx, any_output));
    }
}
