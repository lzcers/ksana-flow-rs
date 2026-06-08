use crate::{Input, NodeId, RunnerId, RunnerKind, StreamSubscriptionFn};
use serde_json::Value;

#[derive(Debug, Clone)]
pub enum FlowEvent {
    NodeStarted(String),
    NodeStreamStarted(String),
    NodeStreamNextMessage(String, Value),
    NodeCompleted(String),
    NodeInMessage(String, Input),
    NodeOutMessage(String, Value),
    NodeError(String, String),
    FlowStarted,
    FlowPaused,
    FlowResumed,
    FlowStopped,
    FlowFinished,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubgraphFrame {
    pub runner_id: RunnerId,
    pub parent_node_id: NodeId,
}

#[derive(Debug, Clone)]
pub struct FlowEventEnvelope {
    pub runner_id: RunnerId,
    pub runner_kind: RunnerKind,
    pub parent_runner_id: Option<RunnerId>,
    pub parent_node_id: Option<NodeId>,
    pub subgraph_path: Vec<SubgraphFrame>,
    pub event: FlowEvent,
}

pub enum TaskEvent {
    Next(NodeId, Value),
    Completed(NodeId, Option<Value>),
    Error(NodeId, String),
    Stream(NodeId, StreamSubscriptionFn),
}
