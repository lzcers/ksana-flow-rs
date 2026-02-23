use super::input::extract_input_string;
use crate::agent::{ChatCapability, ChatModel, DeepSeekProvider, Message, OpenRouterProvider};
use crate::prompt::build_user_prompt;
use async_trait::async_trait;
use flow::{Context, Input, Node, Output};
use serde_json::Value;
use std::sync::Arc;

pub struct LLMNode {
    chat_model: ChatModel,
    model: String,
    system_prompt: String,
    user_prompt_template: String,
}

impl LLMNode {
    pub fn new(sys_prompt: &str, user_tmpl: &str, model: &str) -> Self {
        dotenv::dotenv().ok();

        let mut chat_model = ChatModel::new();

        if model.contains('/') {
            let provider =
                OpenRouterProvider::from_env().expect("Failed to create OpenRouter provider");
            chat_model.add_model_provider(model, Arc::new(provider));
        } else {
            let provider =
                DeepSeekProvider::from_env().expect("Failed to create DeepSeek provider");
            chat_model.add_models_for_provider(
                &["deepseek-chat", "deepseek-reasoner"],
                Arc::new(provider),
            );
        }

        chat_model
            .set_active_model(model)
            .expect("Failed to set active model");

        Self {
            chat_model,
            model: model.to_owned(),
            system_prompt: sys_prompt.to_owned(),
            user_prompt_template: user_tmpl.to_owned(),
        }
    }
}

#[async_trait]
impl Node for LLMNode {
    async fn run(&mut self, _ctx: &Context, input: &Input) -> Result<Output, String> {
        let input = extract_input_string(input);
        let prompt = build_user_prompt(&self.user_prompt_template, &input);

        let mut messages = Vec::new();
        if !self.system_prompt.is_empty() {
            messages.push(Message::system(&self.system_prompt));
        }
        messages.push(Message::user(&prompt));

        let result = self
            .chat_model
            .chat(messages)
            .await
            .map_err(|e| e.to_string())?;

        Ok(Value::String(result.content).into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use flow::Input;
    use serde_json::Value;
    use std::collections::HashMap;
    use tokio::runtime::Runtime;

    #[test]
    #[ignore]
    fn test_llm_node() {
        dotenv::dotenv().ok();
        let runtime = Runtime::new().expect("Failed to create tokio runtime");
        runtime.block_on(async {
            let ctx = Context::new();
            let mut node = LLMNode::new("", "", "deepseek-reasoner");
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
            let mut node = LLMNode::new(
                "You are a helpful translator.",
                "Translate this to English: {input}",
                "deepseek-reasoner",
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
