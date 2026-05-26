use async_trait::async_trait;
use flow::{Context, Input, Node, Output};
use serde_json::Value;

pub struct TextNode {
    #[allow(dead_code)]
    id: String,
    text: String,
}

impl TextNode {
    pub fn new(id: String, text: String) -> Self {
        Self { id, text }
    }
}

#[async_trait]
impl Node for TextNode {
    async fn run(&mut self, _ctx: &Context, input: &Input) -> Result<Output, String> {
        let in_text = match input.get_any() {
            None => String::new(),
            Some(Value::String(s)) => s.clone(),
            Some(Value::Null) => String::new(),
            Some(v) => v.to_string(),
        };
        let output = if in_text.is_empty() {
            self.text.clone()
        } else {
            in_text
        };
        Ok(Value::String(output).into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::collections::HashMap;
    use tokio::runtime::Runtime;

    #[test]
    fn test_text_node() {
        let runtime = Runtime::new().expect("Failed to create tokio runtime");
        runtime.block_on(async {
            let ctx = Context::new();
            let mut node = TextNode::new("node1".to_string(), "default text".to_string());

            // Test with empty input
            let inputs: HashMap<String, Value> = HashMap::new();
            let output = node.run(&ctx, &Input::new(inputs)).await.unwrap();
            assert_eq!(output.get().and_then(|v| v.as_str()), Some("default text"));

            // Test with provided input
            let mut inputs: HashMap<String, Value> = HashMap::new();
            inputs.insert("test".to_string(), Value::String("input text".to_string()));
            let output = node.run(&ctx, &Input::new(inputs)).await.unwrap();
            assert_eq!(output.get().and_then(|v| v.as_str()), Some("input text"));

            let mut inputs: HashMap<String, Value> = HashMap::new();
            inputs.insert("test".to_string(), json!({"a": 1}));
            let output = node.run(&ctx, &Input::new(inputs)).await.unwrap();
            assert_eq!(output.get().and_then(|v| v.as_str()), Some("{\"a\":1}"));

            let mut inputs: HashMap<String, Value> = HashMap::new();
            inputs.insert("test".to_string(), json!([1, 2]));
            let output = node.run(&ctx, &Input::new(inputs)).await.unwrap();
            assert_eq!(output.get().and_then(|v| v.as_str()), Some("[1,2]"));

            let mut inputs: HashMap<String, Value> = HashMap::new();
            inputs.insert("test".to_string(), json!(["a", "b"]));
            let output = node.run(&ctx, &Input::new(inputs)).await.unwrap();
            assert_eq!(output.get().and_then(|v| v.as_str()), Some("[\"a\",\"b\"]"));

            let mut inputs: HashMap<String, Value> = HashMap::new();
            inputs.insert("test".to_string(), json!(1));
            let output = node.run(&ctx, &Input::new(inputs)).await.unwrap();
            assert_eq!(output.get().and_then(|v| v.as_str()), Some("1"));

            let mut inputs: HashMap<String, Value> = HashMap::new();
            inputs.insert("test".to_string(), Value::Null);
            let output = node.run(&ctx, &Input::new(inputs)).await.unwrap();
            assert_eq!(output.get().and_then(|v| v.as_str()), Some("default text"));
        });
    }
}
