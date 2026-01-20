use async_trait::async_trait;
use flow::{Context, SimpleNode};

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
impl SimpleNode for TextNode {
    type In = String;
    type Out = String;

    async fn run(&mut self, ctx: &Context, input: Self::In) -> Self::Out {
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
    use tokio::runtime::Runtime;

    #[test]
    fn test_text_node() {
        let runtime = Runtime::new().expect("Failed to create tokio runtime");
        runtime.block_on(async {
            let ctx = Context::new();
            let mut node = TextNode::new("node1".to_string(), "default text".to_string());

            // Test with empty input
            let output = node.run(&ctx, "".to_string()).await;
            assert_eq!(output, "default text");

            // Test with provided input
            let output = node.run(&ctx, "input text".to_string()).await;
            assert_eq!(output, "input text");
        });
    }
}
