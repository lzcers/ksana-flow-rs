use crate::{NodeId, NodeInputs, SendableAny, StreamSubscriptionFn};

pub enum FlowEvent {
    NodeStarted(String),
    NodeCompleted(String),
    NodeError(String, String),
    NodeInMessage(String, NodeInputs),
    NodeOutMessage(String, Box<dyn SendableAny>),
    NodeStreamStarted(String),
    NodeStreamNextMessage(String, Box<dyn SendableAny>),
    FlowPaused,
    FlowResumed,
    FlowStopped,
    FlowFinished,
}

pub enum TaskEvent {
    Next(NodeId, Box<dyn SendableAny>),
    Completed(NodeId, Option<Box<dyn SendableAny>>),
    Error(NodeId, String),
    Stream(NodeId, StreamSubscriptionFn),
}
