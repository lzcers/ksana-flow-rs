use async_trait::async_trait;
use flow::{Context, Node, NodeInputs};

pub struct TextNode {
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
    type Out = String;

    async fn run(&mut self, ctx: &Context, inputs: NodeInputs) -> Self::Out {
        let input = inputs
            .get_any()
            .and_then(|any| any.as_ref().as_any().downcast_ref::<String>())
            .cloned()
            .unwrap_or_default();

        let output = if input.is_empty() {
            self.text.clone()
        } else {
            input
        };
        output
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
        let runtime = Runtime::new().expect("Failed to create tokio runtime");
        runtime.block_on(async {
            let ctx = Context::new();
            let mut node = TextNode::new("node1".to_string(), "default text".to_string());

            // Test with empty input
            let output = node.run(&ctx, NodeInputs::new(HashMap::new())).await;
            assert_eq!(output, "default text");

            // Test with provided input
            let mut inputs = HashMap::new();
            inputs.insert(
                "test".to_string(),
                Box::new("input text".to_string()) as Box<dyn flow::SendableAny>,
            );
            let output = node.run(&ctx, NodeInputs::new(inputs)).await;
            assert_eq!(output, "input text");
        });
    }
}
