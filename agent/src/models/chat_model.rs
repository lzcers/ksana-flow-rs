use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use futures::stream::{BoxStream, StreamExt};

use crate::{
    agents::ToolDef,
    core::{Message, MessageRole},
    models::{ChatCapability, ChatChunk, ChatError},
    providers::{Provider, Request, Response},
};

pub struct ChatModel {
    model_providers: HashMap<String, Arc<dyn Provider>>,
    active_model: Option<String>,
}

impl ChatModel {
    pub fn new() -> Self {
        Self {
            model_providers: HashMap::new(),
            active_model: None,
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

    fn get_provider(&self, model_name: &str) -> Result<&Arc<dyn Provider>, ChatError> {
        self.model_providers
            .get(model_name)
            .ok_or_else(|| ChatError::ModelNotFound(model_name.to_owned()))
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
        let request = Request::new(model_name, msg).with_tools(tools);

        let response: Response = provider
            .send_request("/chat/completions", &request, model_name)
            .await?;

        let choice = response
            .choices
            .into_iter()
            .next()
            .ok_or(ChatError::NoResponse)?;

        match choice.message.role {
            MessageRole::Assistant => Ok(Message::Assistant {
                content: choice.message.content,
                tool_calls: choice.message.tool_calls,
            }),
            MessageRole::User => Ok(Message::User {
                content: choice.message.content,
            }),
            MessageRole::System => Ok(Message::System {
                content: choice.message.content,
            }),
            MessageRole::Tool => Ok(Message::Tool {
                tool_call_id: choice.message.tool_call_id.unwrap_or_default(),
                content: choice.message.content,
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
        let request = Request::new(model_name, msgs)
            .with_stream(true)
            .with_tools(tools);

        let stream = provider
            .stream_request("/chat/completions", request, model_name)
            .await?;

        Ok(stream
            .map(|response| {
                if let Some(choice) = response.choices.first() {
                    let content = choice.delta.content.clone().unwrap_or_default();
                    let is_finished = choice.finish_reason.is_some();
                    ChatChunk {
                        content,
                        is_finished,
                        finish_reason: choice.finish_reason.clone(),
                        tool_calls: choice.delta.tool_calls.clone(),
                    }
                } else {
                    ChatChunk {
                        content: String::new(),
                        is_finished: true,
                        finish_reason: Some("no_choices".to_string()),
                        tool_calls: None,
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
    use crate::providers::{DeepSeekProvider, OpenRouterProvider};

    #[tokio::test]
    async fn test_chat_with_deepseek_chat() {
        dotenv::dotenv().ok();

        let provider = match DeepSeekProvider::from_env() {
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

        let provider = match DeepSeekProvider::from_env() {
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
    async fn test_chat_with_openrouter_gemini() {
        dotenv::dotenv().ok();

        let provider = match OpenRouterProvider::from_env() {
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

        let provider = match OpenRouterProvider::from_env() {
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
        let deepseek_provider = Arc::new(DeepSeekProvider::new("dummy_key"));
        let openrouter_provider = Arc::new(OpenRouterProvider::new("dummy_key"));

        let mut model = ChatModel::new();

        model.add_models_for_provider(&["deepseek-chat", "deepseek-reasoner"], deepseek_provider);
        model.add_model_provider("google/gemini-3-pro-preview", openrouter_provider);

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
        let provider = Arc::new(DeepSeekProvider::new("dummy_key"));
        let mut model = ChatModel::new();
        model.add_model_provider("deepseek-chat", provider);

        let result = model.set_active_model("deepseek-chat");
        assert!(result.is_ok());
        assert_eq!(model.active_model, Some("deepseek-chat".to_string()));

        let result = model.set_active_model("non-existent-model");
        assert!(result.is_err());
        assert!(matches!(result, Err(ChatError::ModelNotFound(_))));
    }
}
