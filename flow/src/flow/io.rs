use std::collections::HashMap;

use serde_json::Value;

use super::{NodeId, ReactiveStream};

#[derive(Clone)]
pub struct Input {
    values: HashMap<NodeId, Value>,
}

impl Input {
    pub fn new(values: HashMap<NodeId, Value>) -> Self {
        Self { values }
    }
    pub fn get(&self, key: &NodeId) -> Option<&Value> {
        self.values.get(key)
    }
    pub fn get_str(&self, key: &str) -> Option<&Value> {
        self.values.get(key)
    }
    pub fn get_as<T: serde::de::DeserializeOwned>(&self, key: &NodeId) -> Option<T> {
        self.values
            .get(key)
            .and_then(|v| serde_json::from_value(v.clone()).ok())
    }
    pub fn get_str_as<T: serde::de::DeserializeOwned>(&self, key: &str) -> Option<T> {
        self.values
            .get(key)
            .and_then(|v| serde_json::from_value(v.clone()).ok())
    }
    pub fn get_any(&self) -> Option<&Value> {
        if let Some(v) = self.values.get("external_start") {
            return Some(v);
        }
        self.values
            .iter()
            .min_by_key(|(k, _)| k.as_str())
            .map(|(_, v)| v)
    }
    pub fn get_any_as<T: serde::de::DeserializeOwned>(&self) -> Option<T> {
        self.get_any()
            .and_then(|v| serde_json::from_value(v.clone()).ok())
    }
    pub fn get_values(&self) -> &HashMap<NodeId, Value> {
        &self.values
    }
}

pub struct Output {
    value: Option<Value>,
    stream: Option<ReactiveStream>,
}

impl Output {
    pub fn new(value: Option<Value>) -> Self {
        Self {
            value,
            stream: None,
        }
    }
    pub fn get(&self) -> Option<&Value> {
        self.value.as_ref()
    }
    pub fn get_as<T: serde::de::DeserializeOwned>(&self) -> Option<T> {
        self.value
            .as_ref()
            .and_then(|v| serde_json::from_value(v.clone()).ok())
    }
    pub fn set(&mut self, value: Value) {
        self.value = Some(value);
    }
    pub fn is_stream(&self) -> bool {
        self.stream.is_some()
    }
    pub fn get_stream(&self) -> Option<&ReactiveStream> {
        self.stream.as_ref()
    }
    pub fn set_stream(&mut self, stream: ReactiveStream) {
        self.stream = Some(stream);
    }
    pub fn into_value(self) -> Option<Value> {
        self.value
    }
    pub fn into_stream(self) -> Option<ReactiveStream> {
        self.stream
    }
}

impl Into<Output> for Value {
    fn into(self) -> Output {
        Output {
            value: Some(self),
            stream: None,
        }
    }
}
