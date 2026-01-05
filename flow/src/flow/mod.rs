pub mod builder;
pub mod runner;

use async_trait::async_trait;
use dashmap::DashMap;
use serde_json::Value;
use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

type NodeId = String;

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

#[async_trait]
pub trait Node: Send + Sync {
    type In: Send;
    type Out: Clone + Send;
    async fn run(&mut self, ctx: &Context, input: Self::In) -> Self::Out;
}

pub type EdgeCondition<Out> = Box<dyn Fn(&Context, &Out) -> bool>;

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

pub trait AnyEdge: Any {
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
        let input = input.into_any().downcast::<N::In>().map_err(|_| {
            "Type mismatch: input type does not match node's expected type".to_string()
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
}
