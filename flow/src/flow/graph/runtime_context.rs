use std::{any::Any, sync::Arc};

use dashmap::DashMap;
use serde_json::Value;

// 节点运行上下文
// Context 内部是一个并发安全的结构，因此 Context 只要能 Clone 就行
#[derive(Clone)]
pub struct Context {
    data: Arc<DashMap<String, Value>>,
    any: Arc<DashMap<String, Arc<dyn Any + Send + Sync>>>,
    parent: Option<Arc<Context>>,
}

impl Context {
    pub fn new() -> Self {
        Self {
            data: Arc::new(DashMap::new()),
            any: Arc::new(DashMap::new()),
            parent: None,
        }
    }

    pub fn child(&self) -> Self {
        Self {
            data: Arc::new(DashMap::new()),
            any: Arc::new(DashMap::new()),
            parent: Some(Arc::new(self.clone())),
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

    pub fn set_any<T: Any + Send + Sync>(&self, key: impl Into<String>, value: T) {
        self.any.insert(key.into(), Arc::new(value));
    }

    pub fn get_any<T: Any + Send + Sync>(&self, key: &str) -> Option<Arc<T>> {
        self.any
            .get(key)
            .and_then(|v| v.clone().downcast::<T>().ok())
            .or_else(|| self.parent.as_ref().and_then(|p| p.get_any::<T>(key)))
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

        let mut any_keys: Vec<String> = self.any.iter().map(|kv| kv.key().clone()).collect();
        if let Some(parent) = self.parent.as_ref() {
            any_keys.extend(parent.any.iter().map(|kv| kv.key().clone()));
        }
        any_keys.sort();
        any_keys.dedup();
        f.debug_struct("Context")
            .field("data_keys", &data_keys)
            .field("any_keys", &any_keys)
            .finish()
    }
}
