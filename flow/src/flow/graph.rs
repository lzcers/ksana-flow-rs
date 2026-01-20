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

#[async_trait]
pub trait Node {
    type In;
    type Out: SendableAny; // 因为 Out 的值会被 Clone 分发到下游节点中
    async fn run(&mut self, ctx: &Context, input: Self::In) -> Self::Out;
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
        input: Box<dyn SendableAny>,
    ) -> Result<Box<dyn SendableAny>, String>;
}

#[async_trait]
impl<N: Node + Any + Send + Sync> AnyNode for N
where
    N::In: Send,
{
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    async fn run(
        &mut self,
        ctx: &Context,
        input: Box<dyn SendableAny>,
    ) -> Result<Box<dyn SendableAny>, String> {
        // let input_is_unit = input.as_ref().as_any().downcast_ref::<()>().is_some();

        let input_any = if TypeId::of::<N::In>() == TypeId::of::<()>() {
            Box::new(()) as Box<dyn Any>
        } else if TypeId::of::<N::In>() == TypeId::of::<StreamSubscriptionFn>() && input.is_stream()
        {
            match input.into_stream_subscriber() {
                Ok(sub) => Box::new(sub) as Box<dyn Any>,
                Err(i) => i.into_any(),
            }
        }
        // else if TypeId::of::<N::In>() == TypeId::of::<String>() && input_is_unit {
        // 对于真实输入是 () 但 N::In 为 String 类型的情况，我们需要转换为 ""
        // 存在这种情况是因为某些节点定义了 String 输入，但是也可以直接以该节点作为起始节点，不接输入直接运行
        // 对于定义了输入类型，但是又可以实际无输入情况下作为起始节点执行的节点，应该在注册节点时同时注册输入输出类型
        // 并在 set_start 节点时配置输入
        //     Box::new("".to_owned()) as Box<dyn Any>
        // }
        else {
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
