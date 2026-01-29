use async_trait::async_trait;
use flow::{Context, Node, NodeInputs, SendableAny};

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
    async fn run(
        &mut self,
        _ctx: &Context,
        inputs: NodeInputs,
    ) -> Result<Box<dyn SendableAny>, String> {
        let input = inputs
            .get_any()
            .and_then(|a| a.downcast_ref::<String>())
            .cloned()
            .unwrap_or_default();
        let output = if input.is_empty() {
            self.text.clone()
        } else {
            input
        };
        Ok(Box::new(output))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use flow::Context;
    use flow::NodeInputs;
    use std::collections::HashMap;
    use tokio::runtime::Runtime;

    #[test]
    fn test_text_node() {
        fn unwrap_any<'a>(mut any: &'a dyn std::any::Any) -> &'a dyn std::any::Any {
            loop {
                let Some(inner) = any.downcast_ref::<Box<dyn flow::SendableAny>>() else {
                    return any;
                };
                any = inner.as_ref().as_any();
            }
        }

        let runtime = Runtime::new().expect("Failed to create tokio runtime");
        runtime.block_on(async {
            let ctx = Context::new();
            let mut node = TextNode::new("node1".to_string(), "default text".to_string());

            // Test with empty input
            let inputs: HashMap<String, Box<dyn flow::SendableAny>> = HashMap::new();
            let output = node.run(&ctx, NodeInputs::new(inputs)).await.unwrap();
            let s = unwrap_any(output.as_any())
                .downcast_ref::<String>()
                .cloned()
                .unwrap_or_default();
            assert_eq!(s, "default text");

            // Test with provided input
            let mut inputs: HashMap<String, Box<dyn flow::SendableAny>> = HashMap::new();
            inputs.insert("test".to_string(), Box::new("input text".to_string()));
            let output = node.run(&ctx, NodeInputs::new(inputs)).await.unwrap();
            let s = unwrap_any(output.as_any())
                .downcast_ref::<String>()
                .cloned()
                .unwrap_or_default();
            assert_eq!(s, "input text");
        });
    }
}
