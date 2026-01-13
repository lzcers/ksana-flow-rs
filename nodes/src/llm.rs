use async_trait::async_trait;
use flow::Node;
use rig::{
    agent::Agent,
    client::{CompletionClient, ProviderClient},
    completion::Prompt,
    providers::deepseek::{self, CompletionModel},
};
pub struct LLMNode {
    llm: Agent<CompletionModel>,
}

impl LLMNode {
    pub fn new(model: Option<String>, system_prompt: Option<String>) -> Self {
        dotenv::dotenv().ok();
        let client = deepseek::Client::from_env();
        let model_name = model.unwrap_or("deepseek-chat".to_string());
        let mut builder = client.agent(&model_name);

        if let Some(sp) = system_prompt {
            if !sp.is_empty() {
                builder = builder.preamble(&sp);
            }
        }

        let llm = builder.build();
        Self { llm }
    }
}
#[async_trait]
impl Node for LLMNode {
    type In = String;

    type Out = String;

    async fn run(&mut self, _ctx: &flow::Context, input: Self::In) -> Self::Out {
        let prompt = input;
        self.llm.prompt(&prompt).await.expect("LLM prompt failed")
    }
}

#[cfg(test)]
mod tests {
    use flow::Context;
    use tokio::runtime::Runtime;

    use super::*;

    #[test]
    fn test_llm_node() {
        dotenv::dotenv().ok();
        let runtime = Runtime::new().expect("Failed to create tokio runtime");
        runtime.block_on(async {
            let ctx = Context::new();
            let mut node = LLMNode::new(None, None);
            let input = "你好".to_owned();
            eprintln!("input: {}", &input);
            let output = node.run(&ctx, input).await;
            eprintln!("output: {}", output);
            assert!(!output.is_empty());
        });
    }

    #[test]
    fn test_llm_node_with_template() {
        dotenv::dotenv().ok();
        let runtime = Runtime::new().expect("Failed to create tokio runtime");
        runtime.block_on(async {
            let ctx = Context::new();
            let mut node = LLMNode::new(None, Some("You are a helpful translator.".to_string()));
            let input = "你好".to_owned();
            eprintln!("input: {}", &input);
            let output = node.run(&ctx, input).await;
            eprintln!("output: {}", output);
            assert!(!output.is_empty());
        });
    }
}
