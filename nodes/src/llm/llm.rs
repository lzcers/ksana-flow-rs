use super::{agent::LlmAgent, input::extract_input_string};
use crate::prompt::build_user_prompt;
use async_trait::async_trait;
use flow::{Context, Input, Node, Output};
use serde_json::Value;

pub struct LLMNode {
    llm: LlmAgent,
    #[allow(dead_code)]
    model: String,
    #[allow(dead_code)]
    system_prompt: String,
    user_prompt_template: String,
}

impl LLMNode {
    pub fn new(sys_prompt: &str, user_tmpl: &str, model: &str) -> Self {
        let llm = LlmAgent::new(sys_prompt, model);

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
    async fn run(
        &mut self,
        _ctx: &Context,
        input: &Input,
    ) -> Result<Output, String> {
        let input = extract_input_string(input);
        let prompt = build_user_prompt(&self.user_prompt_template, &input);

        let out = self
            .llm
            .prompt(&prompt)
            .await
            .unwrap_or("llm request failed".to_owned());
        Ok(Value::String(out).into())
    }
}

#[cfg(test)]
mod tests {
    use flow::Input;
    use serde_json::Value;
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

            let mut inputs: HashMap<String, Value> = HashMap::new();
            inputs.insert("test".to_string(), Value::String(input));

            let output = node.run(&ctx, &Input::new(inputs)).await.unwrap();
            eprintln!("output: {:?}", output.get());
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

            let mut inputs: HashMap<String, Value> = HashMap::new();
            inputs.insert("test".to_string(), Value::String(input));

            let output = node.run(&ctx, &Input::new(inputs)).await.unwrap();
            eprintln!("output: {:?}", output.get());
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

            let mut inputs: HashMap<String, Value> = HashMap::new();
            inputs.insert("test".to_string(), Value::String(input));

            let output = node.run(&ctx, &Input::new(inputs)).await.unwrap();
            eprintln!("output: {:?}", output.get());
        });
    }
}
