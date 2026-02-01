use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;

use super::{
    SubgraphExecutor,
    graph::Node,
    io::{Input, Output},
    keys::INPUT_EXTERNAL_START,
    runtime_context::Context,
};

pub struct SubgraphNode {
    pub executor: SubgraphExecutor,
}

#[async_trait]
impl Node for SubgraphNode {
    async fn run(&mut self, ctx: &Context, input: &Input) -> Result<Output, String> {
        let v = if let Some(v) = input.get_str(INPUT_EXTERNAL_START) {
            v.clone()
        } else {
            let m = input.get_values();
            if m.is_empty() {
                Value::Null
            } else if m.len() == 1 {
                m.values().next().cloned().unwrap_or(Value::Null)
            } else {
                let mut obj = serde_json::Map::new();
                for (k, v) in m {
                    obj.insert(k.clone(), v.clone());
                }
                Value::Object(obj)
            }
        };

        let out = self
            .executor
            .execute(v, ctx)
            .await
            .map_err(|e| e.to_string())?;
        Ok(out.into())
    }
}

pub struct SubgraphStartNode;

#[async_trait]
impl Node for SubgraphStartNode {
    async fn run(&mut self, _ctx: &Context, input: &Input) -> Result<Output, String> {
        Ok(input.get_any().cloned().unwrap_or(Value::Null).into())
    }
}

pub struct SubgraphInNode {
    pub key: String,
}

#[async_trait]
impl Node for SubgraphInNode {
    async fn run(&mut self, _ctx: &Context, input: &Input) -> Result<Output, String> {
        let v = input.get_any().cloned().unwrap_or(Value::Null);
        if let Value::Object(map) = v {
            Ok(map.get(&self.key).cloned().unwrap_or(Value::Null).into())
        } else {
            Ok(v.into())
        }
    }
}

pub struct SubgraphEndNode;

#[async_trait]
impl Node for SubgraphEndNode {
    async fn run(&mut self, _ctx: &Context, input: &Input) -> Result<Output, String> {
        let m = input.get_values();
        if m.is_empty() {
            return Ok(Value::Null.into());
        }
        if m.len() == 1 {
            return Ok(m.values().next().cloned().unwrap_or(Value::Null).into());
        }
        let mut obj = serde_json::Map::new();
        for (k, v) in m {
            obj.insert(k.clone(), v.clone());
        }
        Ok(Value::Object(obj).into())
    }
}

pub fn pack_inputs_to_object(input: &Input) -> Value {
    if let Some(v) = input.get_str(INPUT_EXTERNAL_START) {
        return v.clone();
    }
    let m = input.get_values();
    if m.is_empty() {
        return Value::Null;
    }
    if m.len() == 1 {
        return m.values().next().cloned().unwrap_or(Value::Null);
    }
    let mut obj = serde_json::Map::new();
    for (k, v) in m {
        obj.insert(k.clone(), v.clone());
    }
    Value::Object(obj)
}

pub fn input_from_object(value: Value) -> Input {
    let mut map = HashMap::new();
    map.insert(INPUT_EXTERNAL_START.to_owned(), value);
    Input::new(map)
}
