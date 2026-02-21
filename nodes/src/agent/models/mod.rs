use super::providers::CompletionProvider;
use crate::agent::core::Message;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ModelError {}
struct ChatModel<T: CompletionProvider> {
    provider: T,
    is_stream: bool,
}

impl<T: CompletionProvider> ChatModel<T> {
    pub fn new(provider: T, is_stream: bool) -> Self {
        Self {
            provider,
            is_stream,
        }
    }
    pub async fn chat(&self, msg: Message) -> Result<Message, ModelError> {
        todo!()
    }
}
