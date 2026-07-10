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

const ROUTED_INPUT_ENVELOPE: &str = "__ksana_flow_subgraph_input";
const ROUTED_INPUT_VERSION: &str = "version";
const ROUTED_INPUT_KIND: &str = "kind";
const ROUTED_INPUT_VALUES: &str = "values";
const ROUTED_INPUT_KIND_VALUE: &str = "routed";

/// 父图中的子图容器节点；负责打包输入并等待子 Runner 返回出口值。
pub struct SubgraphNode {
    pub executor: SubgraphExecutor,
}

#[async_trait]
impl Node for SubgraphNode {
    async fn run(&mut self, ctx: &Context, input: &Input) -> Result<Output, String> {
        // 父图上游输入使用带标签的信封，避免业务 Object 与路由表产生歧义。
        let v = pack_inputs_to_object(input);
        let out = self
            .executor
            .execute(v, ctx)
            .await
            .map_err(|e| e.to_string())?;
        Ok(out.into())
    }
}

/// 子图统一入口，把 `INPUT_EXTERNAL_START` 中的值交给内部代理节点。
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

/// 一条外部来源在子图内部的输入代理。
///
/// 带标签的父图输入按来源 ID 路由；未标记值按直接调用的单值输入透传。
pub struct SubgraphInNode {
    pub key: String,
}

#[async_trait]
impl Node for SubgraphInNode {
    async fn run(&mut self, _ctx: &Context, input: &Input) -> Result<Output, String> {
        let v = input.get_any().cloned().unwrap_or(Value::Null);
        if let Some(map) = routed_input_values(&v) {
            Ok(map.get(&self.key).cloned().unwrap_or(Value::Null).into())
        } else {
            Ok(v.into())
        }
    }
}

/// 子图统一出口：单路结果直接返回，多路结果按来源节点 ID 组成对象。
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

/// 把父图输入归一化为子 Runner 的单个入口值。
///
/// 外部显式输入保持原值，供直接调用 `SubgraphExecutor` 时作为单值透传。
/// 来自父图上游的输入（包括单路）编码为带版本标签的路由信封，由内部代理按来源 ID
/// 提取。业务 JSON Object 因此不会被误认为路由表。
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
    // 即使只有一路也保留来源 ID；代理节点不再需要从 Value 的 JSON 类型猜测协议。
    let mut values = serde_json::Map::new();
    for (k, v) in m {
        values.insert(k.clone(), v.clone());
    }
    routed_input_envelope(values)
}

fn routed_input_envelope(values: serde_json::Map<String, Value>) -> Value {
    let envelope = serde_json::Map::from_iter([
        (ROUTED_INPUT_VERSION.to_owned(), Value::from(1)),
        (
            ROUTED_INPUT_KIND.to_owned(),
            Value::String(ROUTED_INPUT_KIND_VALUE.to_owned()),
        ),
        (ROUTED_INPUT_VALUES.to_owned(), Value::Object(values)),
    ]);
    Value::Object(serde_json::Map::from_iter([(
        ROUTED_INPUT_ENVELOPE.to_owned(),
        Value::Object(envelope),
    )]))
}

fn routed_input_values(value: &Value) -> Option<&serde_json::Map<String, Value>> {
    let outer = value.as_object()?;
    if outer.len() != 1 {
        return None;
    }
    let envelope = outer.get(ROUTED_INPUT_ENVELOPE)?.as_object()?;
    if envelope.len() != 3
        || envelope.get(ROUTED_INPUT_VERSION)?.as_u64() != Some(1)
        || envelope.get(ROUTED_INPUT_KIND)?.as_str() != Some(ROUTED_INPUT_KIND_VALUE)
    {
        return None;
    }
    envelope.get(ROUTED_INPUT_VALUES)?.as_object()
}

/// 把一个入口值包装成可直接传给子图 start 节点的 `Input`。
pub fn input_from_object(value: Value) -> Input {
    let mut map: HashMap<String, Value> = HashMap::new();
    map.insert(INPUT_EXTERNAL_START.to_owned(), value);
    Input::new(map)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    async fn run_inbound_proxy(key: &str, value: Value) -> Value {
        let mut node = SubgraphInNode {
            key: key.to_owned(),
        };
        node.run(&Context::new(), &input_from_object(value))
            .await
            .expect("inbound proxy should run")
            .into_value()
            .expect("inbound proxy should produce a value")
    }

    #[tokio::test]
    async fn single_object_from_parent_is_not_mistaken_for_routing_map() {
        let business_value = json!({"name": "akashic", "enabled": true});
        let parent_input = Input::new(HashMap::from([(
            "source-a".to_owned(),
            business_value.clone(),
        )]));

        let packed = pack_inputs_to_object(&parent_input);

        assert_eq!(run_inbound_proxy("source-a", packed).await, business_value);
    }

    #[tokio::test]
    async fn single_scalar_from_parent_keeps_its_value() {
        let parent_input = Input::new(HashMap::from([("source-a".to_owned(), json!(42))]));

        let packed = pack_inputs_to_object(&parent_input);

        assert_eq!(run_inbound_proxy("source-a", packed).await, json!(42));
    }

    #[tokio::test]
    async fn untagged_object_from_direct_executor_input_is_a_single_value() {
        let business_value = json!({"name": "akashic", "enabled": true});

        assert_eq!(
            run_inbound_proxy("any-proxy", business_value.clone()).await,
            business_value
        );
    }

    #[tokio::test]
    async fn multiple_parent_sources_are_routed_by_source_id() {
        let parent_input = Input::new(HashMap::from([
            ("source-a".to_owned(), json!({"value": "a"})),
            ("source-b".to_owned(), json!({"value": "b"})),
        ]));

        let packed = pack_inputs_to_object(&parent_input);

        assert_eq!(
            run_inbound_proxy("source-b", packed).await,
            json!({"value": "b"})
        );
    }
}
