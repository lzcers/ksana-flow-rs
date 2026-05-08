use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use futures::stream::{BoxStream, StreamExt};

use crate::{
    agent::ToolDef,
    core::{Message, MessageRole},
    models::{ChatCapability, ChatChunk, ChatError},
    providers::{Provider, Request, Response},
};

#[derive(Clone)]
pub struct ChatModel {
    model_providers: HashMap<String, Arc<dyn Provider>>,
    active_model: Option<String>,
    output_json: bool,
    reasoning_effort: Option<String>,
    thinking_enabled: Option<bool>,
}

impl Default for ChatModel {
    fn default() -> Self {
        Self::new()
    }
}

impl ChatModel {
    pub fn new() -> Self {
        Self {
            model_providers: HashMap::new(),
            active_model: None,
            output_json: false,
            reasoning_effort: None,
            thinking_enabled: None,
        }
    }

    pub fn add_model_provider(&mut self, model_name: &str, provider: Arc<dyn Provider>) {
        self.model_providers
            .entry(model_name.to_owned())
            .or_insert(provider);
    }

    pub fn add_models_for_provider(&mut self, model_names: &[&str], provider: Arc<dyn Provider>) {
        for model_name in model_names {
            self.add_model_provider(model_name, provider.clone());
        }
    }

    pub fn set_active_model(&mut self, model_name: &str) -> Result<(), ChatError> {
        if !self.model_providers.contains_key(model_name) {
            return Err(ChatError::ModelNotFound(model_name.to_owned()));
        }
        self.active_model = Some(model_name.to_owned());
        Ok(())
    }

    pub fn get_provider(&self, model_name: &str) -> Result<&Arc<dyn Provider>, ChatError> {
        self.model_providers
            .get(model_name)
            .ok_or_else(|| ChatError::ModelNotFound(model_name.to_owned()))
    }
    pub fn set_output_json(&mut self, output_json: bool) {
        self.output_json = output_json;
    }

    pub fn set_reasoning_effort(&mut self, reasoning_effort: impl Into<String>) {
        self.reasoning_effort = Some(reasoning_effort.into());
    }

    pub fn set_thinking_enabled(&mut self, enabled: bool) {
        self.thinking_enabled = Some(enabled);
    }
}

#[async_trait]
impl ChatCapability for ChatModel {
    async fn chat(
        &self,
        msg: Vec<Message>,
        tools: Option<Vec<ToolDef>>,
    ) -> Result<Message, ChatError> {
        let model_name = self
            .active_model
            .as_ref()
            .ok_or_else(|| ChatError::ModelNotFound("No active model set".to_string()))?;

        let provider = self.get_provider(model_name)?;
        let mut request = Request::new(model_name, msg).with_tools(tools);

        if let Some(reasoning_effort) = &self.reasoning_effort {
            request = request.with_reasoning_effort(reasoning_effort.clone());
        }

        if let Some(enabled) = self.thinking_enabled {
            request = request.with_thinking(enabled);
        }

        let response: Response = provider.chat(request).await?;

        let choice = response
            .choices
            .into_iter()
            .next()
            .ok_or(ChatError::NoResponse)?;

        match choice.message.role {
            MessageRole::Assistant => Ok(Message::Assistant {
                content: choice.message.content.unwrap_or_default(),
                reasoning_content: choice.message.reasoning_content,
                tool_calls: choice.message.tool_calls,
            }),
            MessageRole::User => Ok(Message::User {
                content: choice.message.content.unwrap_or_default(),
            }),
            MessageRole::System => Ok(Message::System {
                content: choice.message.content.unwrap_or_default(),
            }),
            MessageRole::Tool => Ok(Message::Tool {
                tool_call_id: choice.message.tool_call_id.unwrap_or_default(),
                content: choice.message.content.unwrap_or_default(),
            }),
        }
    }

    async fn chat_stream(
        &self,
        msgs: Vec<Message>,
        tools: Option<Vec<ToolDef>>,
    ) -> Result<BoxStream<'static, ChatChunk>, ChatError> {
        let model_name = self
            .active_model
            .as_ref()
            .ok_or_else(|| ChatError::ModelNotFound("No active model set".to_string()))?;

        let provider = self.get_provider(model_name)?;
        let mut request = Request::new(model_name, msgs)
            .with_stream(true)
            .with_stream_usage(true)
            .with_tools(tools);

        if self.output_json {
            request = request.with_response_format_json();
        }

        if let Some(reasoning_effort) = &self.reasoning_effort {
            request = request.with_reasoning_effort(reasoning_effort.clone());
        }

        if let Some(enabled) = self.thinking_enabled {
            request = request.with_thinking(enabled);
        }

        let stream = provider.chat_stream(request).await?;

        Ok(stream
            .map(|response| {
                if let Some(choice) = response.choices.first() {
                    let content = choice.delta.content.clone().unwrap_or_default();
                    let reasoning_content =
                        choice.delta.reasoning_content.clone().unwrap_or_default();
                    let is_finished = choice.finish_reason.is_some();
                    ChatChunk {
                        content,
                        reasoning_content,
                        is_finished,
                        finish_reason: choice.finish_reason.clone(),
                        tool_calls: choice.delta.tool_calls.clone(),
                        usage: response.usage.map(Into::into),
                    }
                } else {
                    ChatChunk {
                        content: String::new(),
                        reasoning_content: String::new(),
                        is_finished: true,
                        finish_reason: Some("no_choices".to_string()),
                        tool_calls: None,
                        usage: response.usage.map(Into::into),
                    }
                }
            })
            .boxed())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::Message;
    use crate::providers::{
        deepseek_provider, deepseek_provider_from_env, openrouter_provider,
        openrouter_provider_from_env,
    };

    #[tokio::test]
    async fn test_chat_with_deepseek_chat() {
        dotenv::dotenv().ok();

        let provider = match deepseek_provider_from_env() {
            Ok(p) => Arc::new(p),
            Err(_) => {
                eprintln!("DEEPSEEK_API_KEY not set, skipping test");
                return;
            }
        };

        let mut model = ChatModel::new();
        model.add_models_for_provider(&["deepseek-chat", "deepseek-reasoner"], provider);

        if let Err(e) = model.set_active_model("deepseek-chat") {
            eprintln!("Failed to set active model: {}", e);
            return;
        }

        let msg = Message::user("Say 'Hello, world!' in one sentence.");

        let result = model.chat(vec![msg], None).await;
        assert!(result.is_ok());

        let message = result.unwrap();
        if let Message::Assistant { content, .. } = message {
            println!("Response: {:?}", content);
            assert!(!content.is_empty());
        } else {
            panic!("Expected Assistant message");
        }
    }

    #[tokio::test]
    async fn test_chat_stream_with_deepseek_chat() {
        dotenv::dotenv().ok();

        let provider = match deepseek_provider_from_env() {
            Ok(p) => Arc::new(p),
            Err(_) => {
                eprintln!("DEEPSEEK_API_KEY not set, skipping test");
                return;
            }
        };

        let mut model = ChatModel::new();
        model.add_models_for_provider(&["deepseek-chat", "deepseek-reasoner"], provider);

        if let Err(e) = model.set_active_model("deepseek-chat") {
            eprintln!("Failed to set active model: {}", e);
            return;
        }

        let msg = Message::user("Count from 1 to 3, each number on a new line.");

        let result = model.chat_stream(vec![msg], None).await;
        assert!(result.is_ok());

        let mut stream = result.unwrap();
        let mut full_content = String::new();

        while let Some(chunk) = stream.next().await {
            print!("{}", chunk.content);
            full_content.push_str(&chunk.content);
            if chunk.is_finished {
                println!("\nFinish reason: {:?}", chunk.finish_reason);
            }
        }

        assert!(!full_content.is_empty());
    }

    #[tokio::test]
    async fn test_chat_with_deepseek_reasoner() {
        dotenv::dotenv().ok();

        let provider = match deepseek_provider_from_env() {
            Ok(p) => Arc::new(p),
            Err(_) => {
                eprintln!("DEEPSEEK_API_KEY not set, skipping test");
                return;
            }
        };

        let mut model = ChatModel::new();
        model.add_models_for_provider(&["deepseek-chat", "deepseek-reasoner"], provider);

        if let Err(e) = model.set_active_model("deepseek-reasoner") {
            eprintln!("Failed to set active model: {}", e);
            return;
        }

        let msg = Message::user("What is 15 + 27? Please think step by step.");

        let result = model.chat(vec![msg], None).await;
        assert!(result.is_ok());

        let message = result.unwrap();
        if let Message::Assistant {
            content,
            reasoning_content,
            ..
        } = message
        {
            println!("Response: {:?}", content);
            assert!(!content.is_empty());
            // 推理模型应该返回推理内容
            if let Some(rc) = reasoning_content {
                println!("Reasoning content length: {}", rc.len());
                assert!(!rc.is_empty(), "Reasoning content should not be empty");
            }
        } else {
            panic!("Expected Assistant message");
        }
    }

    #[tokio::test]
    async fn test_chat_stream_with_deepseek_reasoner() {
        dotenv::dotenv().ok();

        let provider = match deepseek_provider_from_env() {
            Ok(p) => Arc::new(p),
            Err(_) => {
                eprintln!("DEEPSEEK_API_KEY not set, skipping test");
                return;
            }
        };

        let mut model = ChatModel::new();
        model.add_models_for_provider(&["deepseek-chat", "deepseek-reasoner"], provider);

        if let Err(e) = model.set_active_model("deepseek-reasoner") {
            eprintln!("Failed to set active model: {}", e);
            return;
        }

        let msg = Message::user("What is 8 * 7? Think step by step.");

        let result = model.chat_stream(vec![msg], None).await;
        assert!(result.is_ok());

        let mut stream = result.unwrap();
        let mut full_content = String::new();
        let mut full_reasoning = String::new();

        while let Some(chunk) = stream.next().await {
            if !chunk.content.is_empty() {
                print!("{}", chunk.content);
                full_content.push_str(&chunk.content);
            }
            if !chunk.reasoning_content.is_empty() {
                full_reasoning.push_str(&chunk.reasoning_content);
            }
            if chunk.is_finished {
                println!("\nFinish reason: {:?}", chunk.finish_reason);
            }
        }

        assert!(!full_content.is_empty());
        // 推理模型应该返回推理内容
        if !full_reasoning.is_empty() {
            println!("\nReasoning content length: {}", full_reasoning.len());
        }
    }

    #[tokio::test]
    async fn test_chat_with_openrouter_gemini() {
        dotenv::dotenv().ok();

        let provider = match openrouter_provider_from_env() {
            Ok(p) => Arc::new(p),
            Err(_) => {
                eprintln!("OPENROUTER_API_KEY not set, skipping test");
                return;
            }
        };

        let mut model = ChatModel::new();
        model.add_model_provider("google/gemini-3-pro-preview", provider);

        if let Err(e) = model.set_active_model("google/gemini-3-pro-preview") {
            eprintln!("Failed to set active model: {}", e);
            return;
        }

        let msg = Message::user("Say 'Hello, world!' in one sentence.");

        let result = model.chat(vec![msg], None).await;
        assert!(result.is_ok());

        let message = result.unwrap();
        if let Message::Assistant { content, .. } = message {
            println!("Response: {:?}", content);
            assert!(!content.is_empty());
        } else {
            panic!("Expected Assistant message");
        }
    }

    #[tokio::test]
    async fn test_chat_stream_with_openrouter_gemini() {
        dotenv::dotenv().ok();

        let provider = match openrouter_provider_from_env() {
            Ok(p) => Arc::new(p),
            Err(_) => {
                eprintln!("OPENROUTER_API_KEY not set, skipping test");
                return;
            }
        };

        let mut model = ChatModel::new();
        model.add_model_provider("google/gemini-3-pro-preview", provider);

        if let Err(e) = model.set_active_model("google/gemini-3-pro-preview") {
            eprintln!("Failed to set active model: {}", e);
            return;
        }

        let msg = Message::user("Count from 1 to 3, each number on a new line.");

        let result = model.chat_stream(vec![msg], None).await;
        assert!(result.is_ok());

        let mut stream = result.unwrap();
        let mut full_content = String::new();

        while let Some(chunk) = stream.next().await {
            print!("{}", chunk.content);
            full_content.push_str(&chunk.content);
            if chunk.is_finished {
                println!("\nFinish reason: {:?}", chunk.finish_reason);
            }
        }

        assert!(!full_content.is_empty());
    }

    #[test]
    fn test_model_provider_mapping() {
        let ds_provider = Arc::new(deepseek_provider("dummy_key"));
        let or_provider = Arc::new(openrouter_provider("dummy_key"));

        let mut model = ChatModel::new();

        model.add_models_for_provider(&["deepseek-chat", "deepseek-reasoner"], ds_provider);
        model.add_model_provider("google/gemini-3-pro-preview", or_provider);

        assert!(model.model_providers.contains_key("deepseek-chat"));
        assert!(model.model_providers.contains_key("deepseek-reasoner"));
        assert!(
            model
                .model_providers
                .contains_key("google/gemini-3-pro-preview")
        );
        assert_eq!(model.model_providers.len(), 3);
    }

    #[test]
    fn test_set_active_model() {
        let provider = Arc::new(deepseek_provider("dummy_key"));
        let mut model = ChatModel::new();
        model.add_model_provider("deepseek-chat", provider);

        let result = model.set_active_model("deepseek-chat");
        assert!(result.is_ok());
        assert_eq!(model.active_model, Some("deepseek-chat".to_string()));

        let result = model.set_active_model("non-existent-model");
        assert!(result.is_err());
        assert!(matches!(result, Err(ChatError::ModelNotFound(_))));
    }

    #[test]
    fn test_set_reasoning_options() {
        let mut model = ChatModel::new();

        model.set_reasoning_effort("high");
        model.set_thinking_enabled(true);

        assert_eq!(model.reasoning_effort.as_deref(), Some("high"));
        assert_eq!(model.thinking_enabled, Some(true));
    }
}
