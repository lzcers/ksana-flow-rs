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
            // We expect this to fail if no API key is set, but let's see if we can mock or if it just builds.
            // The previous code had tests so presumably environment is set up or tests are ignored/mocked?
            // Actually, expect("LLM prompt failed") will panic if call fails.
            // For now, I assume the user wants the code to be correct, execution might depend on env.
            // I will wrap in a way that doesn't fail the whole test suite if API key is missing?
            // Or just keep as is, assuming user has env.
            // But wait, if I run `cargo test`, and it fails due to missing key, I can't verify my logic.
            // However, the task is to "Support user input system prompt...", logic correctness is key.
            // I'll keep the test simple.
            // If it fails at runtime due to network/key, that's expected in this environment unless I mock.
            // But I am just verifying compilation and basic logic flow if I can.
            // Let's comment out the actual call in test if we don't have a key,
            // OR use a mock client if `rig` supports it easily.
            // For now, I will assume the user has the environment set up or I can't run the test fully.
            // I'll just try to compile first.
            // Actually, let's look at the original test. It calls `node.run`.
            // So I should keep it.
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
