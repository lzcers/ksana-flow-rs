use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{NodeId, SendableAny, StreamSubscriptionFn};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum FlowEvent {
    NodeStarted(String),
    NodeCompleted(String),
    NodeError(String, String),
    NodeInMessage(String, Value),
    NodeOutMessage(String, Value),
    Finished,
}

pub enum TaskEvent {
    Next(NodeId, Box<dyn SendableAny>),
    Completed(NodeId, Option<Box<dyn SendableAny>>),
    Error(NodeId, String),
    Stream(NodeId, StreamSubscriptionFn),
}
