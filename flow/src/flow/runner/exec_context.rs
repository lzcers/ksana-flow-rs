use std::sync::Arc;

use super::task_guard::TaskTracker;
use crate::flow::{graph::NodeId, runner::task_guard::TaskGuard};
use crate::OutputPayload;
use crate::observable::Subscription;
use dashmap::DashMap;

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
    node_outputs: DashMap<NodeId, OutputPayload>,
    stream_subscriptions: DashMap<NodeId, Box<dyn Subscription>>,
    tracker: Arc<TaskTracker>,
}

impl ExecutionContext {
    pub fn new() -> Self {
        Self {
            node_states: DashMap::new(),
            node_outputs: DashMap::new(),
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

    pub fn set_output(&self, node_id: NodeId, output: OutputPayload) {
        self.node_outputs.insert(node_id, output);
    }

    pub fn get_output(&self, node_id: &str) -> Option<OutputPayload> {
        self.node_outputs.get(node_id).map(|v| v.clone())
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
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    #[test]
    fn exec_ctx_arc_output_is_shared_by_handle_clone() {
        let exec_ctx = ExecutionContext::new();
        exec_ctx.set_output("n1".to_string(), OutputPayload::shared(vec![1, 2, 3]));

        let out1 = exec_ctx.get_output("n1").unwrap();
        let out2 = exec_ctx.get_output("n1").unwrap();

        match (out1, out2) {
            (
                OutputPayload::Shared { value: a1, .. },
                OutputPayload::Shared { value: a2, .. },
            ) => {
                assert!(Arc::ptr_eq(&a1, &a2));
            }
            _ => panic!("Expected OutputPayload::Shared"),
        }
    }

    struct CloneCounted {
        clones: Arc<AtomicUsize>,
        value: usize,
    }

    impl Clone for CloneCounted {
        fn clone(&self) -> Self {
            self.clones.fetch_add(1, Ordering::SeqCst);
            Self {
                clones: self.clones.clone(),
                value: self.value,
            }
        }
    }

    #[test]
    fn exec_ctx_clone_only_output_is_cloned_on_get() {
        let exec_ctx = ExecutionContext::new();
        let clones = Arc::new(AtomicUsize::new(0));
        exec_ctx.set_output(
            "n1".to_string(),
            OutputPayload::cloned(CloneCounted {
                clones: clones.clone(),
                value: 42,
            }),
        );

        let _ = exec_ctx.get_output("n1").unwrap();
        let _ = exec_ctx.get_output("n1").unwrap();
        assert_eq!(clones.load(Ordering::SeqCst), 2);
    }
}
