use async_trait::async_trait;
use dashmap::DashMap;
use serde_json::Value;
use std::{any::Any, collections::HashMap, sync::Arc};
use tokio::sync::RwLock;

use super::sendable_any::SendableAny;

pub type NodeId = String;

pub struct NodeInputs {
    pub inputs: HashMap<NodeId, Box<dyn SendableAny>>,
}

impl NodeInputs {
    pub fn new(inputs: HashMap<NodeId, Box<dyn SendableAny>>) -> Self {
        Self { inputs }
    }

    pub fn get<T: 'static>(&self, key: &str) -> Option<&T> {
        self.inputs
            .get(key)
            .and_then(|any| any.as_ref().as_any().downcast_ref::<T>())
    }

    pub fn get_any(&self) -> Option<&Box<dyn SendableAny>> {
        self.inputs.values().next()
    }
}

#[async_trait]
pub trait Node {
    type Out: SendableAny;
    async fn run(&mut self, ctx: &Context, inputs: NodeInputs) -> Self::Out;
}

pub type EdgeCondition<Out> = Box<dyn Fn(&Context, &Out) -> bool + Send>;

// 条件边接收一个节点传出的输出引用，根据条件判断是否继续执行下一个节点
pub struct Edge<Out = ()> {
    pub from: String,
    pub to: String,
    pub condition: Option<EdgeCondition<Out>>,
}

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
    fn as_any_mut(&mut self) -> &mut dyn Any;
    async fn run(
        &mut self,
        ctx: &Context,
        inputs: NodeInputs,
    ) -> Result<Box<dyn SendableAny>, String>;
}

#[async_trait]
impl<N: Node + Any + Send + Sync> AnyNode for N {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    async fn run(
        &mut self,
        ctx: &Context,
        inputs: NodeInputs,
    ) -> Result<Box<dyn SendableAny>, String> {
        let result = self.run(ctx, inputs).await;
        Ok(Box::new(result))
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

    pub fn add_node<N: AnyNode>(&mut self, id: &str, node: N) {
        self.nodes.insert(
            id.to_owned(),
            Arc::new(RwLock::new(node)) as Arc<RwLock<dyn AnyNode>>,
        );
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

    pub fn add_arc_node(&mut self, id: &str, node: Arc<RwLock<dyn AnyNode>>) {
        self.nodes.insert(id.to_owned(), node);
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
