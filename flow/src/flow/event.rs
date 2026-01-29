use crate::{Input, NodeId, StreamSubscriptionFn};
use serde_json::Value;

pub enum FlowEvent {
    NodeStarted(String),
    NodeCompleted(String),
    NodeError(String, String),
    NodeInMessage(String, Input),
    NodeOutMessage(String, Value),
    NodeStreamStarted(String),
    NodeStreamNextMessage(String, Value),
    FlowPaused,
    FlowResumed,
    FlowStopped,
    FlowFinished,
}

pub enum TaskEvent {
    Next(NodeId, Value),
    Completed(NodeId, Option<Value>),
    Error(NodeId, String),
    Stream(NodeId, StreamSubscriptionFn),
}
