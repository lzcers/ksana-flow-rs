use async_trait::async_trait;
use dashmap::DashMap;
use serde_json::Value;
use std::collections::HashMap;
use std::{
    any::{Any, TypeId},
    sync::Arc,
};
use tokio::sync::RwLock;

use crate::flow::reactive_stream::StreamSubscriptionFn;

pub type NodeId = String;

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

pub enum TaskEvent {
    Next(NodeId, Box<dyn CloneAny>),
    Completed(NodeId, Option<Box<dyn CloneAny>>),
    Error(NodeId, String),
    Stream(NodeId, StreamSubscriptionFn),
}

#[async_trait]
pub trait Node: Send + Sync {
    type In: Send;
    type Out: CloneAny + Send;
    async fn run(&mut self, ctx: &Context, input: Self::In) -> Self::Out;
}

pub type EdgeCondition<Out> = Box<dyn Fn(&Context, &Out) -> bool + Send + Sync>;

pub struct Edge<Out = ()> {
    pub from: String,
    pub to: String,
    pub condition: Option<EdgeCondition<Out>>,
}

pub trait CloneAny: Any + Send {
    fn clone_box(&self) -> Box<dyn CloneAny>;
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn into_any(self: Box<Self>) -> Box<dyn Any>;
    fn type_name(&self) -> &'static str;
    fn is_stream(&self) -> bool {
        false
    }
    fn into_stream_subscriber(self: Box<Self>) -> Result<StreamSubscriptionFn, Box<dyn CloneAny>>;
}

impl<T: Any + Clone + Send> CloneAny for T {
    fn clone_box(&self) -> Box<dyn CloneAny> {
        Box::new(self.clone())
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }
    fn type_name(&self) -> &'static str {
        std::any::type_name::<T>()
    }
    fn into_stream_subscriber(self: Box<Self>) -> Result<StreamSubscriptionFn, Box<dyn CloneAny>> {
        Err(self)
    }
}

impl Clone for Box<dyn CloneAny> {
    fn clone(&self) -> Self {
        self.as_ref().clone_box()
    }
}

#[async_trait]
pub trait AnyNode: Any + Send + Sync {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    async fn run(
        &mut self,
        ctx: &Context,
        input: Box<dyn CloneAny>,
    ) -> Result<Box<dyn CloneAny>, String>;
}

pub trait AnyEdge: Any + Send + Sync {
    fn as_any(&self) -> &dyn Any;
    fn from(&self) -> &str;
    fn to(&self) -> &str;
    fn check_condition(&self, ctx: &Context, output: &dyn Any) -> bool;
}

#[async_trait]
impl<N: Node + Send + Sync + 'static> AnyNode for N {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    async fn run(
        &mut self,
        ctx: &Context,
        input: Box<dyn CloneAny>,
    ) -> Result<Box<dyn CloneAny>, String> {
        println!(
            "Node Input: {}, Expected: {}",
            input.type_name(),
            std::any::type_name::<N::In>()
        );
        let input_any = if TypeId::of::<N::In>() == TypeId::of::<()>() {
            Box::new(()) as Box<dyn Any>
        } else if TypeId::of::<N::In>() == TypeId::of::<StreamSubscriptionFn>() && input.is_stream()
        {
            match input.into_stream_subscriber() {
                Ok(sub) => Box::new(sub) as Box<dyn Any>,
                Err(i) => i.into_any(),
            }
        } else {
            input.into_any()
        };

        let input = input_any.downcast::<N::In>().map_err(|any| {
            format!(
                "Type mismatch: expected {}, got {:?}",
                std::any::type_name::<N::In>(),
                any.as_ref().type_id()
            )
        })?;
        let result = self.run(ctx, *input).await;
        Ok(Box::new(result))
    }
}

impl<Out: 'static> AnyEdge for Edge<Out> {
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
}

impl Graph {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            edges: HashMap::new(),
        }
    }

    pub fn get_node_ids(&self) -> Vec<String> {
        self.nodes.keys().cloned().collect()
    }

    pub fn add_node<N: Node + Send + Sync + 'static>(&mut self, id: &str, node: N) {
        self.nodes.insert(
            id.to_owned(),
            Arc::new(RwLock::new(node)) as Arc<RwLock<dyn AnyNode>>,
        );
    }

    pub fn add_edge<Out: 'static>(&mut self, edge: Edge<Out>) {
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
        self.edges.remove(id); // remove outgoing edges
        // remove incoming edges
        for edges in self.edges.values_mut() {
            edges.retain(|e| e.to() != id);
        }
    }

    pub fn remove_edge(&mut self, from: &str, to: &str) {
        if let Some(edges) = self.edges.get_mut(from) {
            edges.retain(|e| e.to() != to);
        }
    }
}
