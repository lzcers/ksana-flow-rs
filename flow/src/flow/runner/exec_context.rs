use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use super::task_guard::TaskTracker;
use crate::flow::{graph::NodeId, runner::task_guard::TaskGuard};
use crate::observable::Subscription;
use dashmap::DashMap;
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeState {
    Idle,
    Pending,
    Running,
    Completed,
    Failed,
    Skipped,
}

// 执行上下文
// 存储所有节点的执行状态，为调度器提供调度必要信息
pub struct ExecutionContext {
    node_states: DashMap<NodeId, NodeState>,
    node_outputs: Mutex<HashMap<NodeId, Value>>,
    stream_subscriptions: DashMap<NodeId, Box<dyn Subscription>>,
    tracker: Arc<TaskTracker>,
}

impl ExecutionContext {
    pub fn new() -> Self {
        Self {
            node_states: DashMap::new(),
            node_outputs: Mutex::new(HashMap::new()),
            stream_subscriptions: DashMap::new(),
            // 任务跟踪器
            tracker: Arc::new(TaskTracker::new()),
        }
    }

    pub fn get_task_count(&self) -> usize {
        self.tracker.count()
    }
    pub fn reset_task_count(&self) {
        self.tracker.reset();
    }
    pub fn get_task_tracker_guard(&self) -> TaskGuard {
        TaskGuard::new(self.tracker.clone())
    }
    pub fn set_state(&self, node_id: NodeId, state: NodeState) {
        self.node_states.insert(node_id, state);
    }

    pub fn get_state(&self, node_id: &str) -> Option<NodeState> {
        self.node_states.get(node_id).map(|s| *s)
    }

    pub fn set_output(&self, node_id: NodeId, output: Value) {
        if let Ok(mut map) = self.node_outputs.lock() {
            map.insert(node_id, output);
        }
    }

    pub fn get_output(&self, node_id: &str) -> Option<Value> {
        self.node_outputs
            .lock()
            .ok()
            .and_then(|map| map.get(node_id).cloned())
    }

    pub fn set_stream_subscription(&self, node_id: NodeId, sub: Box<dyn Subscription>) {
        let _ = self.stream_subscriptions.insert(node_id, sub);
    }

    pub fn remove_stream_subscription(&self, node_id: &str) -> Option<Box<dyn Subscription>> {
        self.stream_subscriptions.remove(node_id).map(|(_, v)| v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exec_ctx_get_output_returns_stored_value() {
        let exec_ctx = ExecutionContext::new();
        exec_ctx.set_output("n1".to_string(), serde_json::json!({"x": 1}));

        let out1 = exec_ctx.get_output("n1").unwrap();
        let out2 = exec_ctx.get_output("n1").unwrap();
        assert_eq!(out1, serde_json::json!({"x": 1}));
        assert_eq!(out2, serde_json::json!({"x": 1}));
    }
}
