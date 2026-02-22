use futures::{Stream, StreamExt};

use crate::agent::{
    core::Message,
    models::{ChatCapability, ChatChunk, ChatError},
    providers::{Provider, Request, Response},
};

/// 聊天模型
pub struct ChatModel<T: Provider> {
    provider: T,
    model: String,
}

impl<T: Provider> ChatModel<T> {
    /// 创建新的聊天模型
    pub fn new(provider: T, model: String) -> Self {
        Self { provider, model }
    }
}

impl<T: Provider> ChatCapability for ChatModel<T> {
    async fn chat(&self, msg: &Message) -> Result<Message, ChatError> {
        let messages = vec![msg.clone()];
        let request = Request::new(&self.model, messages);

        let response: Response = self
            .provider
            .send_request("/chat/completions", &request, &self.model)
            .await?;

        let choice = response
            .choices
            .into_iter()
            .next()
            .ok_or(ChatError::NoResponse)?;

        Ok(Message {
            role: choice.message.role,
            content: choice.message.content,
        })
    }

    async fn chat_stream(&self, msg: &Message) -> Result<impl Stream<Item = ChatChunk>, ChatError> {
        let messages = vec![msg.clone()];
        let request = Request::new(&self.model, messages).with_stream(true);

        let stream = self
            .provider
            .stream_request("/chat/completions", request, &self.model)
            .await?;

        Ok(stream.map(|response| {
            if let Some(choice) = response.choices.first() {
                let content = choice.delta.content.clone().unwrap_or_default();
                let is_finished = choice.finish_reason.is_some();
                ChatChunk {
                    content,
                    is_finished,
                    finish_reason: choice.finish_reason.clone(),
                }
            } else {
                ChatChunk {
                    content: String::new(),
                    is_finished: true,
                    finish_reason: Some("no_choices".to_string()),
                }
            }
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::core::Message;
    use crate::agent::providers::DeepSeekProvider;

    #[tokio::test]
    async fn test_chat_with_deepseek() {
        dotenv::dotenv().ok();

        let provider = match DeepSeekProvider::from_env() {
            Ok(p) => p,
            Err(_) => {
                eprintln!("DEEPSEEK_API_KEY not set, skipping test");
                return;
            }
        };

        let model = ChatModel::new(provider, "deepseek-chat".to_string());
        let msg = Message::user("Say 'Hello, world!' in one sentence.");

        let result = model.chat(&msg).await;
        assert!(result.is_ok());

        let message = result.unwrap();
        println!("Response: {}", message.content);
        assert!(!message.content.is_empty());
    }

    #[tokio::test]
    async fn test_chat_stream_with_deepseek() {
        dotenv::dotenv().ok();

        let provider = match DeepSeekProvider::from_env() {
            Ok(p) => p,
            Err(_) => {
                eprintln!("DEEPSEEK_API_KEY not set, skipping test");
                return;
            }
        };

        let model = ChatModel::new(provider, "deepseek-chat".to_string());
        let msg = Message::user("Count from 1 to 3, each number on a new line.");

        let result = model.chat_stream(&msg).await;
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
}
