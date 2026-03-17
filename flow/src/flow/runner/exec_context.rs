use super::logger::log_node_state_change;
use super::task_guard::TaskTracker;
use crate::RunnerId;
use crate::flow::{graph::NodeId, runner::task_guard::TaskGuard};
use crate::observable::Subscription;
use dashmap::DashMap;
use serde_json::Value;
use std::sync::Arc;
use std::time::Instant;

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
    node_outputs: DashMap<NodeId, Value>,
    stream_subscriptions: DashMap<NodeId, Box<dyn Subscription>>,
    tracker: Arc<TaskTracker>,
    runner_id: Option<RunnerId>,
    // 节点执行开始时间，用于计算执行耗时
    node_start_times: DashMap<NodeId, Instant>,
}

impl ExecutionContext {
    pub fn new() -> Self {
        Self {
            node_states: DashMap::new(),
            node_outputs: DashMap::new(),
            stream_subscriptions: DashMap::new(),
            // 任务跟踪器
            tracker: Arc::new(TaskTracker::new()),
            runner_id: None,
            node_start_times: DashMap::new(),
        }
    }

    /// 设置 Runner ID，用于日志记录
    pub fn set_runner_id(&mut self, runner_id: RunnerId) {
        self.runner_id = Some(runner_id);
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
        let old_state = self.node_states.get(&node_id).map(|s| *s);

        // 状态变化时记录日志
        if old_state != Some(state) {
            if let Some(runner_id) = self.runner_id {
                log_node_state_change(&node_id, old_state, state, runner_id);
            }
        }

        // 追踪节点执行时间
        match state {
            NodeState::Running => {
                // 节点开始执行，记录开始时间
                self.node_start_times
                    .insert(node_id.clone(), Instant::now());
            }
            NodeState::Completed | NodeState::Failed => {
                // 节点执行结束，可以在这里获取执行时间
                // 实际日志记录在 runner.rs 中完成，那里可以访问 runner_id
                self.node_start_times.remove(&node_id);
            }
            _ => {}
        }

        self.node_states.insert(node_id, state);
    }

    pub fn get_state(&self, node_id: &str) -> Option<NodeState> {
        self.node_states.get(node_id).map(|s| *s)
    }

    pub fn set_output(&self, node_id: NodeId, output: Value) {
        self.node_outputs.insert(node_id, output);
    }

    pub fn get_output(&self, node_id: &str) -> Option<Value> {
        self.node_outputs.get(node_id).map(|v| v.value().clone())
    }

    pub fn set_stream_subscription(&self, node_id: NodeId, sub: Box<dyn Subscription>) {
        let _ = self.stream_subscriptions.insert(node_id, sub);
    }

    pub fn remove_stream_subscription(&self, node_id: &str) -> Option<Box<dyn Subscription>> {
        self.stream_subscriptions.remove(node_id).map(|(_, v)| v)
    }

    /// 获取节点的执行开始时间（如果节点正在执行）
    pub fn get_node_start_time(&self, node_id: &str) -> Option<Instant> {
        self.node_start_times.get(node_id).map(|t| *t)
    }

    /// 计算节点的执行耗时（如果节点正在执行或刚完成）
    pub fn get_node_elapsed(&self, node_id: &str) -> Option<std::time::Duration> {
        self.get_node_start_time(node_id).map(|t| t.elapsed())
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
