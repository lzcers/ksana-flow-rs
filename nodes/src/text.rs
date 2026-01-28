use async_trait::async_trait;
use flow::{Context, Node, NodeInputs, OutputPayload};

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
    async fn run(&mut self, _ctx: &Context, inputs: NodeInputs) -> Result<OutputPayload, String> {
        let input = inputs
            .get_any()
            .and_then(|p| p.as_any())
            .and_then(|a| a.downcast_ref::<String>())
            .cloned()
            .unwrap_or_default();
        let output = if input.is_empty() {
            self.text.clone()
        } else {
            input
        };
        Ok(OutputPayload::cloned(output))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use flow::Context;
    use flow::NodeInputs;
    use flow::OutputPayload;
    use std::collections::HashMap;
    use tokio::runtime::Runtime;

    #[test]
    fn test_text_node() {
        let runtime = Runtime::new().expect("Failed to create tokio runtime");
        runtime.block_on(async {
            let ctx = Context::new();
            let mut node = TextNode::new("node1".to_string(), "default text".to_string());

            // Test with empty input
            let output = node.run(&ctx, NodeInputs::new(HashMap::new())).await.unwrap();
            let s = output
                .as_any()
                .and_then(|a| a.downcast_ref::<String>())
                .cloned()
                .unwrap_or_default();
            assert_eq!(s, "default text");

            // Test with provided input
            let mut inputs = HashMap::new();
            inputs.insert("test".to_string(), OutputPayload::cloned("input text".to_string()));
            let output = node.run(&ctx, NodeInputs::new(inputs)).await.unwrap();
            let s = output
                .as_any()
                .and_then(|a| a.downcast_ref::<String>())
                .cloned()
                .unwrap_or_default();
            assert_eq!(s, "input text");
        });
    }
}
