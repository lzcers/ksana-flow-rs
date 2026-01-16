use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use tokio::sync::Notify;

// RAII 风格的任务计数器，用于跟踪当前运行中的任务数量
#[derive(Debug)]
pub struct TaskTracker {
    count: AtomicUsize,
    notify: Notify,
}

impl TaskTracker {
    pub fn new() -> Self {
        Self {
            count: AtomicUsize::new(0),
            notify: Notify::new(),
        }
    }

    pub fn increment(&self) {
        self.count.fetch_add(1, Ordering::SeqCst);
    }

    pub fn decrement(&self) {
        let prev = self.count.fetch_sub(1, Ordering::SeqCst);
        if prev == 1 {
            self.notify.notify_waiters();
        }
    }

    pub fn count(&self) -> usize {
        self.count.load(Ordering::SeqCst)
    }

    pub async fn await_notify(&self) {
        self.notify.notified().await;
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
}

impl Drop for TaskGuard {
    fn drop(&mut self) {
        self.tracker.decrement();
    }
}
