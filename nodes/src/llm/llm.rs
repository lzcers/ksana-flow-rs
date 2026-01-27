use crate::prompt::build_user_prompt;
use async_trait::async_trait;
use flow::{Node, NodeInputs};
use rig::{
    agent::Agent,
    client::{CompletionClient, ProviderClient},
    completion::Prompt,
    providers::{
        deepseek::{self, CompletionModel},
        openrouter::{self, CompletionModel as OpenRouterCompletionModel},
    },
};

enum LLMAgent {
    DeepSeek(Agent<CompletionModel>),
    OpenRouter(Agent<OpenRouterCompletionModel>),
}

impl LLMAgent {
    async fn prompt(&self, prompt: &str) -> Result<String, String> {
        match self {
            LLMAgent::DeepSeek(agent) => agent.prompt(prompt).await.map_err(|e| e.to_string()),
            LLMAgent::OpenRouter(agent) => agent.prompt(prompt).await.map_err(|e| e.to_string()),
        }
    }
}

pub struct LLMNode {
    llm: LLMAgent,
    #[allow(dead_code)]
    model: String,
    #[allow(dead_code)]
    system_prompt: String,
    user_prompt_template: String,
}

impl LLMNode {
    pub fn new(sys_prompt: &str, user_tmpl: &str, model: &str) -> Self {
        dotenv::dotenv().ok();

        let use_openrouter = model.contains('/');

        let llm = if use_openrouter {
            let client = openrouter::Client::from_env();
            let mut builder = client.agent(model);
            if !sys_prompt.is_empty() {
                builder = builder.preamble(sys_prompt);
            }
            LLMAgent::OpenRouter(builder.build())
        } else {
            let client = deepseek::Client::from_env();
            let mut builder = client.agent(model);
            if !sys_prompt.is_empty() {
                builder = builder.preamble(sys_prompt);
            }
            LLMAgent::DeepSeek(builder.build())
        };

        Self {
            llm,
            model: model.to_owned(),
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

        let prompt = build_user_prompt(&self.user_prompt_template, &input);

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
    use rig::providers::deepseek::DEEPSEEK_REASONER;

    #[test]
    #[ignore]
    fn test_llm_node() {
        dotenv::dotenv().ok();
        let runtime = Runtime::new().expect("Failed to create tokio runtime");
        runtime.block_on(async {
            let ctx = Context::new();
            let mut node = LLMNode::new("", "", DEEPSEEK_REASONER);
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
    #[ignore]
    fn test_open_router_llm_node() {
        dotenv::dotenv().ok();
        let runtime = Runtime::new().expect("Failed to create tokio runtime");
        runtime.block_on(async {
            let ctx = Context::new();
            let mut node = LLMNode::new("", "", "google/gemini-3-pro-preview");
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
    #[ignore]
    fn test_llm_node_with_template() {
        dotenv::dotenv().ok();
        let runtime = Runtime::new().expect("Failed to create tokio runtime");
        runtime.block_on(async {
            let ctx = Context::new();
            // Template with {input} placeholder
            let mut node = LLMNode::new(
                "You are a helpful translator.",
                "Translate this to English: {input}",
                DEEPSEEK_REASONER,
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
