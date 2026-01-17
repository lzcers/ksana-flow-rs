use async_trait::async_trait;
use flow::Node;
use rig::{
    agent::Agent,
    client::{CompletionClient, ProviderClient},
    completion::Prompt,
    providers::deepseek::{self, CompletionModel},
    providers::openrouter::{self, CompletionModel as OpenRouterCompletionModel},
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
        let model_name = "deepseek-chat";
        let mut builder = client.agent(model_name);

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
    type In = String;
    type Out = String;

    async fn run(&mut self, _ctx: &flow::Context, input: Self::In) -> Self::Out {
        let prompt = if !input.is_empty() {
            if self.user_prompt_template.contains("{input}") {
                self.user_prompt_template.replace("{input}", &input)
            } else {
                input
            }
        } else {
            // Input is empty, use the template as is
            self.user_prompt_template.clone()
        };

        if prompt.is_empty() {
            return "".to_string();
        }

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
            let mut node = LLMNode::new("", "");
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
            // Template with {input} placeholder
            let mut node = LLMNode::new(
                "You are a helpful translator.",
                "Translate this to English: {input}",
            );
            let input = "你好".to_owned();
            eprintln!("input: {}", &input);
            let output = node.run(&ctx, input).await;
            eprintln!("output: {}", output);
            assert!(!output.is_empty());
        });
    }

    #[test]
    fn test_llm_node_empty_input() {
        dotenv::dotenv().ok();
        let runtime = Runtime::new().expect("Failed to create tokio runtime");
        runtime.block_on(async {
            let ctx = Context::new();
            // Template without placeholder, used when input is empty
            let mut node = LLMNode::new("", "Tell me a joke");
            let input = "".to_owned();
            eprintln!("input: {}", &input);
            let output = node.run(&ctx, input).await;
            eprintln!("output: {}", output);
            assert!(!output.is_empty());
        });
    }
}
