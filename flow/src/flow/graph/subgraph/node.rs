use super::{
    SubgraphExecutor,
    graph::Node,
    io::{Input, Output},
    keys::INPUT_EXTERNAL_START,
};
use crate::Context;
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;

pub struct SubgraphNode {
    pub executor: SubgraphExecutor,
}

#[async_trait]
impl Node for SubgraphNode {
    async fn run(&mut self, ctx: &Context, input: &Input) -> Result<Output, String> {
        // 子图的起始节点接收外部输入（多路是对象，单路就直接传递）
        let v = pack_inputs_to_object(input);
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
        // 从外部来的输入会被包装成对象
        // 多路 {"INPUT_EXTERNAL_START": {[SourceId: Value]}}
        // 单路 {"INPUT_EXTERNAL_START": Value}
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
        // 多路你就从对象中取，单路就直接返回
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

// 把子图输入打包成对象
// 对于子图来说，其暴露在父图中只是一个 SubgraphNode 节点，
// 而父图在调用子图时，会把输入作为一个整体传递给子图，
// 子图内部会根据 INPUT_EXTERNAL_START 来判断是否是外部输入。
pub fn pack_inputs_to_object(input: &Input) -> Value {
    // 直接运行节点指定输入的情况，此时相当于子图的 INPUT_EXTERNAL_START 被指定了，
    // 它不是由上游节点出发的
    if let Some(v) = input.get_str(INPUT_EXTERNAL_START) {
        return v.clone();
    }
    let m = input.get_values();
    if m.is_empty() {
        return Value::Null;
    }
    // 子图只有一路输入时，直接返回该输入
    if m.len() == 1 {
        return m.values().next().cloned().unwrap_or(Value::Null);
    }
    // 多路输入时，打包成对象
    let mut obj = serde_json::Map::new();
    for (k, v) in m {
        obj.insert(k.clone(), v.clone());
    }
    Value::Object(obj)
}

pub fn input_from_object(value: Value) -> Input {
    let mut map: HashMap<String, Value> = HashMap::new();
    map.insert(INPUT_EXTERNAL_START.to_owned(), value);
    Input::new(map)
}
