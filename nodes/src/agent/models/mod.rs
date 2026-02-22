mod chat_model;
mod gen_img_model;

pub use chat_model::ChatModel;
pub use gen_img_model::GenImgModel;

use futures::Stream;
use thiserror::Error;

use crate::agent::{core::Message, providers::ProviderError};

/// 聊天错误类型
#[derive(Debug, Error)]
pub enum ChatError {
    #[error("Provider error: {0}")]
    Provider(#[from] ProviderError),
    #[error("No response from model")]
    NoResponse,
    #[error("Stream error: {0}")]
    StreamError(String),
}

/// 聊天流式响应片段
#[derive(Debug, Clone)]
pub struct ChatChunk {
    /// 本次流式返回的片段内容
    pub content: String,
    /// 标记是否是最后一个片段
    pub is_finished: bool,
    /// 结束原因（比如 "stop" / "length"）
    pub finish_reason: Option<String>,
}

/// 聊天能力 trait
pub trait ChatCapability {
    /// 非流式聊天
    async fn chat(&self, msg: &Message) -> Result<Message, ChatError>;
    /// 流式聊天
    async fn chat_stream(&self, msg: &Message) -> Result<impl Stream<Item = ChatChunk>, ChatError>;
}

/// 图片生成响应
#[derive(Debug, Clone)]
pub struct GenImgResponse {
    /// 图片 URL 列表
    pub image_urls: Vec<String>,
}

// 生图能力
pub trait GenImgCapability {
    /// 生成图片
    async fn gen_img(&self, msg: &Message) -> Result<GenImgResponse, ChatError>;
}
