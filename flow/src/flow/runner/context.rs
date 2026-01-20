use crate::flow::{graph::NodeId, sendable_any::SendableAny};
use dashmap::DashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeState {
    Pending,
    Running,
    Completed,
    Failed,
    Skipped,
}

pub struct ExecutionContext {
    pub node_states: DashMap<NodeId, NodeState>,
    pub node_outputs: DashMap<NodeId, Box<dyn SendableAny>>,
}

impl ExecutionContext {
    pub fn new() -> Self {
        Self {
            node_states: DashMap::new(),
            node_outputs: DashMap::new(),
        }
    }

    pub fn set_state(&self, node_id: NodeId, state: NodeState) {
        self.node_states.insert(node_id, state);
    }

    pub fn get_state(&self, node_id: &str) -> Option<NodeState> {
        self.node_states.get(node_id).map(|s| *s)
    }

    pub fn set_output(&self, node_id: NodeId, output: Box<dyn SendableAny>) {
        self.node_outputs.insert(node_id, output);
    }

    pub fn get_output(&self, node_id: &str) -> Option<Box<dyn SendableAny>> {
        // Clone the Box? SendableAny is a trait.
        // Box<dyn SendableAny> needs to be clonable.
        // SendableAny extends Send + Any + CloneBox.
        // So we can clone it.
        self.node_outputs.get(node_id).map(|v| v.clone())
    }
}
