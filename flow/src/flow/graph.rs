use async_trait::async_trait;
use dashmap::DashMap;
use serde_json::Value;
use std::{
    any::{Any, TypeId},
    collections::HashMap,
    sync::Arc,
};
use tokio::sync::RwLock;

use super::{reactive_stream::StreamSubscriptionFn, sendable_any::SendableAny};

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
            .and_then(|any| any.as_any().downcast_ref::<T>())
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

#[async_trait]
pub trait SimpleNode {
    type In;
    type Out: SendableAny; // 因为 Out 的值会被 Clone 分发到下游节点中
    async fn run(&mut self, ctx: &Context, input: Self::In) -> Self::Out;
}

#[async_trait]
impl<T: SimpleNode + Send + Sync> Node for T
where
    T::In: Send + 'static,
{
    type Out = T::Out;
    async fn run(&mut self, ctx: &Context, inputs: NodeInputs) -> Self::Out {
        // Attempt to find a matching input
        // 1. If In is (), return ()
        // 2. If inputs is empty, and In is (), return ()
        // 3. Try to pick the first input

        if TypeId::of::<T::In>() == TypeId::of::<()>() {
            // Safe to cast to In (which is ())
            // We need to create a dummy input of type T::In
            // Since we know T::In is (), we can just unsafe transmute or Box::new(())
            // But simpler: just create a Box<dyn Any> of () and downcast
            let unit = Box::new(()) as Box<dyn Any>;
            let input = *unit.downcast::<T::In>().unwrap();
            return SimpleNode::run(self, ctx, input).await;
        }

        let first_input = inputs.get_any();

        let input_any = if let Some(input) = first_input {
            if TypeId::of::<T::In>() == TypeId::of::<StreamSubscriptionFn>() && input.is_stream() {
                match input.clone_box().into_stream_subscriber() {
                    Ok(sub) => Box::new(sub) as Box<dyn Any>,
                    Err(i) => i.into_any(),
                }
            } else {
                input.clone_box().into_any()
            }
        } else {
            // No input provided.
            // If T::In is (), handled above.
            // If T::In is something else, this will fail in downcast unless we handle it.
            // For now, let's assume we provide () if empty?
            // Or better, panic with meaningful error?
            Box::new(()) as Box<dyn Any>
        };

        let input = input_any.downcast::<T::In>().map_err(|any| {
            format!(
                "Type mismatch in SimpleNode adapter: expected {}, got {:?}",
                std::any::type_name::<T::In>(),
                any.as_ref().type_id()
            )
        });

        match input {
            Ok(input) => SimpleNode::run(self, ctx, *input).await,
            Err(e) => panic!("{}", e), // We have to panic here because run signature doesn't return Result
                                       // Ideally Node::run should return Result, but that's a bigger change.
                                       // Existing SimpleNode::run doesn't return Result.
                                       // So panic is the only option for type mismatch in adapter.
        }
    }
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

        // Remove incoming edges (edges pointing TO this node)
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
