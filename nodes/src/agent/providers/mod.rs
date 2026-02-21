mod deepseek;
mod utils;
use super::core::{Content, Message, Usage};
use async_trait::async_trait;
use futures::stream::BoxStream;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ModelProviderError {
    #[error("Missing env var: {0}")]
    MissingEnvVar(String),
    #[error("Request failed: {0}")]
    RequestFailed(String),
    #[error("Failed to parse response: {0}")]
    ResponseParseFailed(String),
    #[error("API error {status}: {body}")]
    ApiError { status: u16, body: String },
    #[error("Invalid response: {0}")]
    InvalidResponse(String),
    #[error("Stream not supported")]
    StreamNotSupported,
}
/// 通用完成请求 (聊天、补全等)
pub struct CompletionRequest {
    /// 目标模型名称 (如 "deepseek-chat", "gpt-4")
    pub model: String,
    /// 消息历史
    pub messages: Vec<Message>,
    /// 通用参数
    pub options: CompletionOptions,
    /// 扩展字段：用于供应商特定参数 (如 DeepSeek 的特殊参数)
    /// 这保证了抽象的灵活性，不会因供应商差异而频繁修改 Trait
    pub extensions: std::collections::HashMap<String, serde_json::Value>,
}

pub struct CompletionStreamChunk {
    pub id: String,
    pub model: String,
    pub content: Vec<Content>,
    pub finish_reason: Option<String>,
    pub extensions: std::collections::HashMap<String, serde_json::Value>,
}

pub type CompletionStream = BoxStream<'static, Result<CompletionStreamChunk, ModelProviderError>>;

pub struct CompletionStreamResponse {
    pub stream: CompletionStream,
}

/// 通用完成响应
pub struct CompletionResponse {
    pub id: String,
    pub model: String,
    pub content: Vec<Content>,
    pub usage: Option<Usage>,
    pub finish_reason: String,
    /// 扩展字段：保留供应商特定元数据
    pub extensions: std::collections::HashMap<String, serde_json::Value>,
}

pub struct CompletionOptions {
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub top_p: Option<f32>,
    pub stream: bool,
}
// 对话类型能力
#[async_trait]
pub trait CompletionProvider: Send + Sync {
    fn name(&self) -> &str;
    /// 非流式完成
    async fn complete(
        &self,
        request: CompletionRequest,
    ) -> Result<CompletionResponse, ModelProviderError>;

    /// 流式完成
    async fn complete_stream(
        &self,
        request: CompletionRequest,
    ) -> Result<CompletionStreamResponse, ModelProviderError>;
}
