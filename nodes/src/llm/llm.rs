use super::input::extract_input_string;
use crate::prompt::build_user_prompt;
use agent::{
    core::Message,
    models::{ChatCapability, ChatModel},
    providers::{deepseek_provider_from_env, openrouter_provider_from_env},
};
use async_trait::async_trait;
use flow::{Context, Input, Node, Output, ReactiveStream};
use futures::{StreamExt, stream};
use serde_json::Value;
use std::sync::Arc;

pub struct LLMNode {
    chat_model: ChatModel,
    model: String,
    system_prompt: String,
    user_prompt_template: String,
    stream: bool,
}

impl LLMNode {
    pub fn new(sys_prompt: &str, user_tmpl: &str, model: &str, stream: bool) -> Self {
        dotenv::dotenv().ok();

        let mut chat_model = ChatModel::new();

        let provider = deepseek_provider_from_env().expect("Failed to create DeepSeek provider");
        chat_model
            .add_models_for_provider(&["deepseek-chat", "deepseek-reasoner"], Arc::new(provider));
        if model.contains('/') {
            let provider =
                openrouter_provider_from_env().expect("Failed to create OpenRouter provider");
            chat_model.add_model_provider(model, Arc::new(provider));
        }

        chat_model
            .set_active_model(model)
            .expect("Failed to set active model");

        Self {
            chat_model,
            model: model.to_owned(),
            system_prompt: sys_prompt.to_owned(),
            user_prompt_template: user_tmpl.to_owned(),
            stream,
        }
    }

    fn build_messages(&self, prompt: &str) -> Vec<Message> {
        let mut messages = Vec::new();
        if !self.system_prompt.is_empty() {
            messages.push(Message::system(&self.system_prompt));
        }
        messages.push(Message::user(prompt));
        messages
    }
}

#[async_trait]
impl Node for LLMNode {
    async fn run(&mut self, _ctx: &Context, input: &Input) -> Result<Output, String> {
        let input = extract_input_string(input);
        let prompt = build_user_prompt(&self.user_prompt_template, &input);
        let messages = self.build_messages(&prompt);

        if self.stream {
            let stream_result = self
                .chat_model
                .chat_stream(messages, None)
                .await
                .map_err(|e| e.to_string())?;

            let content_stream = stream::unfold(
                (Box::pin(stream_result), false),
                |(mut source, finished)| async move {
                    if finished {
                        return None;
                    }
                    source.next().await.map(|chunk| {
                        let finished = chunk.is_finished;
                        (Ok::<_, String>(chunk.content), (source, finished))
                    })
                },
            );
            let stream = ReactiveStream::from_stream_with_accumulator(
                content_stream,
                |chunks: Vec<String>| {
                    let full_text = chunks.join("");
                    Some(Value::String(full_text))
                },
            );
            let mut out = Output::new(None);
            out.set_stream(stream);
            Ok(out)
        } else {
            let result = self
                .chat_model
                .chat(messages, None)
                .await
                .map_err(|e| e.to_string())?;

            let content = match result {
                Message::Assistant { content, .. } => content,
                _ => String::new(),
            };

            Ok(Value::String(content).into())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use flow::{Context, TaskEvent};
    use flow::{Input, StreamStartFn, TaskGuard};
    use serde_json::Value;
    use std::collections::HashMap;
    use std::sync::Arc;
    use tokio::runtime::Runtime;

    async fn collect_output(start_stream: StreamStartFn) -> String {
        let (tx, mut rx) = tokio::sync::mpsc::channel(100);
        let ctx = Arc::new(Context::new());
        let guard = TaskGuard::default();
        let _task = start_stream(guard, tx, "test".to_string(), ctx);

        let mut output = String::new();
        while let Some(event) = rx.recv().await {
            match event {
                TaskEvent::Next(_, val) => {
                    if let Some(s) = val.as_str() {
                        output.push_str(s);
                    }
                }
                TaskEvent::Completed(_, _) => break,
                TaskEvent::Error(_, e) => panic!("Stream error: {}", e),
                _ => {}
            }
        }
        output
    }

    #[test]
    #[ignore]
    fn test_llm_node_non_stream() {
        dotenv::dotenv().ok();
        let runtime = Runtime::new().expect("Failed to create tokio runtime");
        runtime.block_on(async {
            let ctx = Context::new();
            let mut node = LLMNode::new("", "", "deepseek-reasoner", false);
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
    fn test_llm_node_stream() {
        dotenv::dotenv().ok();
        let runtime = Runtime::new().expect("Failed to create tokio runtime");
        runtime.block_on(async {
            let ctx = Context::new();
            let mut node = LLMNode::new("", "", "deepseek-chat", true);
            let input = "你好".to_owned();
            eprintln!("input: {}", &input);

            let mut inputs: HashMap<String, Value> = HashMap::new();
            inputs.insert("test".to_string(), Value::String(input));

            let out = node.run(&ctx, &Input::new(inputs)).await.unwrap();
            let stream = out.into_stream().expect("Expected stream output");
            let start_stream = stream.start;
            let output = collect_output(start_stream).await;
            eprintln!("output: {}", output);
            assert!(!output.is_empty());
        });
    }

    #[test]
    #[ignore]
    fn test_llm_node_with_template_non_stream() {
        dotenv::dotenv().ok();
        let runtime = Runtime::new().expect("Failed to create tokio runtime");
        runtime.block_on(async {
            let ctx = Context::new();
            let mut node = LLMNode::new(
                "You are a helpful translator.",
                "Translate this to English: {input}",
                "deepseek-reasoner",
                false,
            );
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
    fn test_llm_node_with_template_stream() {
        dotenv::dotenv().ok();
        let runtime = Runtime::new().expect("Failed to create tokio runtime");
        runtime.block_on(async {
            let ctx = Context::new();
            let mut node = LLMNode::new(
                "You are a helpful translator.",
                "Translate this to English: {input}",
                "deepseek-chat",
                true,
            );
            let input = "你好".to_owned();
            eprintln!("input: {}", &input);

            let mut inputs: HashMap<String, Value> = HashMap::new();
            inputs.insert("test".to_string(), Value::String(input));

            let out = node.run(&ctx, &Input::new(inputs)).await.unwrap();
            let stream = out.into_stream().expect("Expected stream output");
            let start_stream = stream.start;
            let output = collect_output(start_stream).await;
            eprintln!("output: {}", output);
            assert!(!output.is_empty());
        });
    }

    #[test]
    #[ignore]
    fn test_open_router_llm_node_non_stream() {
        dotenv::dotenv().ok();
        let runtime = Runtime::new().expect("Failed to create tokio runtime");
        runtime.block_on(async {
            let ctx = Context::new();
            let mut node = LLMNode::new("", "", "google/gemini-3-pro-preview", false);
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
    fn test_open_router_llm_node_stream() {
        dotenv::dotenv().ok();
        let runtime = Runtime::new().expect("Failed to create tokio runtime");
        runtime.block_on(async {
            let ctx = Context::new();
            let mut node = LLMNode::new("", "", "google/gemini-3-pro-preview", true);
            let input = "你好".to_owned();
            eprintln!("input: {}", &input);

            let mut inputs: HashMap<String, Value> = HashMap::new();
            inputs.insert("test".to_string(), Value::String(input));

            let out = node.run(&ctx, &Input::new(inputs)).await.unwrap();
            let stream = out.into_stream().expect("Expected stream output");
            let start_stream = stream.start;
            let output = collect_output(start_stream).await;
            eprintln!("output: {}", output);
            assert!(!output.is_empty());
        });
    }
}
