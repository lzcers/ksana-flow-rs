mod chat_model;
mod gen_img_model;

use crate::agent::ToolCall;
use crate::core::Usage;
use async_trait::async_trait;
pub use chat_model::ChatModel;
pub use gen_img_model::GenImgModel;

use futures::stream::BoxStream;
use thiserror::Error;

use crate::{agent::ToolDef, core::Message, providers::ProviderError};

/// 聊天错误类型
#[derive(Debug, Error)]
pub enum ChatError {
    #[error("Provider error: {0}")]
    Provider(#[from] ProviderError),
    #[error("No response from model")]
    NoResponse,
    #[error("Stream error: {0}")]
    StreamError(String),
    #[error("Model not found: {0}")]
    ModelNotFound(String),
}

/// 聊天流式响应片段
#[derive(Debug, Clone)]
pub struct ChatChunk {
    /// 本次流式返回的片段内容
    pub content: String,
    /// DeepSeek 推理模式的推理内容片段（如 deepseek-reasoner）
    pub reasoning_content: String,
    /// 标记是否是最后一个片段
    pub is_finished: bool,
    /// 结束原因（比如 "stop" / "length"）
    pub finish_reason: Option<String>,
    /// 流式输出中的工具调用（用于解析增量工具调用）
    pub tool_calls: Option<Vec<ToolCall>>,
    /// 供应商返回的 token 用量
    pub usage: Option<Usage>,
}

/// 聊天能力 trait
#[async_trait]
pub trait ChatCapability {
    /// 非流式聊天
    async fn chat(
        &self,
        msgs: Vec<Message>,
        tools: Option<Vec<ToolDef>>,
    ) -> Result<Message, ChatError>;
    /// 流式聊天
    async fn chat_stream(
        &self,
        msgs: Vec<Message>,
        tools: Option<Vec<ToolDef>>,
    ) -> Result<BoxStream<'static, ChatChunk>, ChatError>;
}

/// 图片生成响应
#[derive(Debug, Clone)]
pub struct GenImgResponse {
    /// 图片 URL 列表
    pub image_urls: Vec<String>,
}

// 生图能力 trait
#[async_trait]
pub trait GenImgCapability {
    /// 生成图片
    async fn gen_img(&self, msgs: Vec<Message>) -> Result<GenImgResponse, ChatError>;
}
