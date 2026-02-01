use std::{
    collections::HashMap,
    fmt::{self, Display},
};

use serde_json::Value;

use crate::ReactiveStream;

use super::{graph::NodeId, keys::INPUT_EXTERNAL_START};

#[derive(Clone, Debug)]
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
        if let Some(v) = self.values.get(INPUT_EXTERNAL_START) {
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

impl Into<Input> for Value {
    fn into(self) -> Input {
        Input::new(HashMap::from([(INPUT_EXTERNAL_START.to_string(), self)]))
    }
}
impl Into<Input> for HashMap<NodeId, Value> {
    fn into(self) -> Input {
        Input::new(self)
    }
}

impl Display for Input {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // 先写入结构体标识，开头换行让格式更清晰
        write!(f, "Input {{\n")?;

        // 遍历HashMap的键值对，逐个格式化写入（缩进4个空格，美观）
        for (node_id, value) in &self.values {
            write!(f, "    node_id: {}, value: {}\n", node_id, value)?;
        }

        // 写入结构体结束符，与开头匹配
        write!(f, "}}")
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
