use std::sync::Arc;

use dashmap::DashMap;
use serde_json::Value;
use tokio::sync::{OnceCell, broadcast, mpsc};

use crate::FlowEvent;
use crate::flow::runner::RunnerCommand;

// 节点运行上下文
// Context 内部是一个并发安全的结构，因此 Context 只要能 Clone 就行
#[derive(Clone)]
pub struct Context {
    data: Arc<DashMap<String, Value>>,
    parent: Option<Arc<Context>>,
    // 外部事件发送通道
    // 放上下文上主要是方便透传到子图
    flow_event_tx: Arc<OnceCell<mpsc::Sender<FlowEvent>>>,
    runner_cmd_tx: Arc<OnceCell<broadcast::Sender<RunnerCommand>>>,
}

impl Context {
    pub fn new() -> Self {
        Self {
            data: Arc::new(DashMap::new()),
            parent: None,
            flow_event_tx: Arc::new(OnceCell::new()),
            runner_cmd_tx: Arc::new(OnceCell::new()),
        }
    }

    pub fn child(&self) -> Self {
        Self {
            data: Arc::new(DashMap::new()),
            parent: Some(Arc::new(self.clone())),
            flow_event_tx: self.flow_event_tx.clone(),
            runner_cmd_tx: self.runner_cmd_tx.clone(),
        }
    }

    pub fn set(&self, key: impl Into<String>, value: impl serde::Serialize) {
        let value = serde_json::to_value(value).expect("Failed to serialize value");
        self.data.insert(key.into(), value);
    }

    pub fn get<T: serde::de::DeserializeOwned>(&self, key: &str) -> Option<T> {
        self.data
            .get(key)
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .or_else(|| self.parent.as_ref().and_then(|p| p.get::<T>(key)))
    }

    pub fn set_flow_event_sender(&self, sender: mpsc::Sender<FlowEvent>) {
        let _ = self.flow_event_tx.set(sender);
    }
    pub fn get_flow_event_sender(&self) -> Option<mpsc::Sender<FlowEvent>> {
        self.flow_event_tx.get().cloned()
    }
    pub fn get_flow_event_sender_ref(&self) -> Option<&mpsc::Sender<FlowEvent>> {
        self.flow_event_tx.get()
    }

    pub fn set_runner_command_sender(&self, sender: broadcast::Sender<RunnerCommand>) {
        let _ = self.runner_cmd_tx.set(sender);
    }
    pub fn get_runner_command_sender(&self) -> Option<broadcast::Sender<RunnerCommand>> {
        self.runner_cmd_tx.get().cloned()
    }
}

impl std::fmt::Debug for Context {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut data_keys: Vec<String> = self.data.iter().map(|kv| kv.key().clone()).collect();
        if let Some(parent) = self.parent.as_ref() {
            data_keys.extend(parent.data.iter().map(|kv| kv.key().clone()));
        }
        data_keys.sort();
        data_keys.dedup();

        f.debug_struct("Context")
            .field("data_keys", &data_keys)
            .finish()
    }
}
