use async_trait::async_trait;
use flow::{Node, NodeInputs};
use rig::{
    agent::Agent,
    client::{CompletionClient, ProviderClient},
    completion::Prompt,
    providers::deepseek::{self, CompletionModel, DEEPSEEK_REASONER},
};

pub struct LLMNode {
    llm: Agent<CompletionModel>,
    #[allow(dead_code)]
    system_prompt: String,
    user_prompt_template: String,
}

impl LLMNode {
    pub fn new(sys_prompt: &str, user_tmpl: &str) -> Self {
        dotenv::dotenv().ok();
        // Initialize the DeepSeek client from environment variables
        let client = deepseek::Client::from_env();
        let mut builder = client.agent(DEEPSEEK_REASONER);

        // Handle system prompt
        if !sys_prompt.is_empty() {
            builder = builder.preamble(&sys_prompt);
        }

        let llm = builder.build();

        Self {
            llm,
            system_prompt: sys_prompt.to_owned(),
            user_prompt_template: user_tmpl.to_owned(),
        }
    }
}

#[async_trait]
impl Node for LLMNode {
    type Out = String;

    async fn run(&mut self, _ctx: &flow::Context, inputs: NodeInputs) -> Self::Out {
        let input = inputs
            .get_any()
            .and_then(|any| any.as_ref().as_any().downcast_ref::<String>())
            .cloned()
            .unwrap_or_default();

        let prompt = if !input.is_empty() {
            if self.user_prompt_template.contains("{input}") {
                self.user_prompt_template.replace("{input}", &input)
            } else {
                input
            }
        } else {
            self.user_prompt_template.clone()
        };

        self.llm
            .prompt(&prompt)
            .await
            .unwrap_or("llm request failed".to_owned())
    }
}

#[cfg(test)]
mod tests {
    use flow::Context;
    use flow::NodeInputs;
    use std::collections::HashMap;
    use tokio::runtime::Runtime;

    use super::*;

    #[test]
    fn test_llm_node() {
        dotenv::dotenv().ok();
        let runtime = Runtime::new().expect("Failed to create tokio runtime");
        runtime.block_on(async {
            let ctx = Context::new();
            let mut node = LLMNode::new("", "");
            let input = "你好".to_owned();
            eprintln!("input: {}", &input);

            let mut inputs = HashMap::new();
            inputs.insert(
                "test".to_string(),
                Box::new(input) as Box<dyn flow::SendableAny>,
            );

            let output = node.run(&ctx, NodeInputs::new(inputs)).await;
            eprintln!("output: {}", output);
        });
    }

    #[test]
    fn test_llm_node_with_template() {
        dotenv::dotenv().ok();
        let runtime = Runtime::new().expect("Failed to create tokio runtime");
        runtime.block_on(async {
            let ctx = Context::new();
            // Template with {input} placeholder
            let mut node = LLMNode::new(
                "You are a helpful translator.",
                "Translate this to English: {input}",
            );
            let input = "你好".to_owned();
            eprintln!("input: {}", &input);

            let mut inputs = HashMap::new();
            inputs.insert(
                "test".to_string(),
                Box::new(input) as Box<dyn flow::SendableAny>,
            );

            let output = node.run(&ctx, NodeInputs::new(inputs)).await;
            eprintln!("output: {}", output);
        });
    }
}
