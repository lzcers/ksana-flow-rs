use crate::{NodeId, NodeInputs, OutputPayload, StreamSubscriptionFn};

pub enum FlowEvent {
    NodeStarted(String),
    NodeCompleted(String),
    NodeError(String, String),
    NodeInMessage(String, NodeInputs),
    NodeOutMessage(String, OutputPayload),
    NodeStreamStarted(String),
    NodeStreamNextMessage(String, OutputPayload),
    FlowPaused,
    FlowResumed,
    FlowStopped,
    FlowFinished,
}

pub enum TaskEvent {
    Next(NodeId, OutputPayload),
    Completed(NodeId, Option<OutputPayload>),
    Error(NodeId, String),
    Stream(NodeId, StreamSubscriptionFn),
}
