use crate::{Input, NodeId, RunnerId, RunnerKind, StreamSubscriptionFn};
use serde_json::Value;

/// Runner 对外发布的生命周期和数据事件。
///
/// 事件只描述发生了什么；所属 Runner、父子关系和子图路径由
/// `FlowEventEnvelope` 补充。
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

/// 一个子图 Runner 在嵌套执行路径中的位置。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubgraphFrame {
    pub runner_id: RunnerId,
    pub parent_node_id: NodeId,
}

/// Controller 事件通道传递的完整事件信封。
#[derive(Debug, Clone)]
pub struct FlowEventEnvelope {
    pub runner_id: RunnerId,
    pub runner_kind: RunnerKind,
    pub parent_runner_id: Option<RunnerId>,
    pub parent_node_id: Option<NodeId>,
    pub subgraph_path: Vec<SubgraphFrame>,
    pub event: FlowEvent,
}

/// Executor/ReactiveStream 发给 Runner 的内部推进事件。
///
/// Runner 串行消费这些事件并更新 `ExecutionContext`，因此节点任务不得直接
/// 修改调度状态。
pub enum TaskEvent {
    Next(NodeId, Value),
    Completed(NodeId, Option<Value>),
    Error(NodeId, String),
    Stream(NodeId, StreamSubscriptionFn),
}
