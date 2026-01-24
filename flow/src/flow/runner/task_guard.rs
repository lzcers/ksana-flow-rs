use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

// RAII 风格的任务计数器，用于跟踪当前运行中的任务数量
#[derive(Debug)]
pub struct TaskTracker {
    count: AtomicUsize,
}

impl TaskTracker {
    pub fn new() -> Self {
        Self {
            count: AtomicUsize::new(0),
        }
    }

    pub fn increment(&self) {
        self.count.fetch_add(1, Ordering::SeqCst);
    }

    pub fn decrement(&self) {
        self.count.fetch_sub(1, Ordering::SeqCst);
    }
    pub fn reset(&self) {
        self.count.store(0, Ordering::SeqCst);
    }
    pub fn count(&self) -> usize {
        self.count.load(Ordering::SeqCst)
    }
}

pub struct TaskGuard {
    tracker: Arc<TaskTracker>,
}

impl TaskGuard {
    pub fn new(tracker: Arc<TaskTracker>) -> Self {
        tracker.increment();
        Self { tracker }
    }
    pub fn default() -> Self {
        Self::new(Arc::new(TaskTracker::new()))
    }
}

impl Drop for TaskGuard {
    fn drop(&mut self) {
        self.tracker.decrement();
    }
}
