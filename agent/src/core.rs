use serde::{Deserialize, Serialize};

use crate::agent::ToolCall;

/// 用量
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
    pub prompt_cache_hit_tokens: Option<u32>,
    pub prompt_cache_miss_tokens: Option<u32>,
}

impl From<crate::providers::Usage> for Usage {
    fn from(value: crate::providers::Usage) -> Self {
        Self {
            prompt_tokens: value.prompt_tokens,
            completion_tokens: value.completion_tokens,
            total_tokens: value.total(),
            prompt_cache_hit_tokens: value.prompt_cache_hit_tokens,
            prompt_cache_miss_tokens: value.prompt_cache_miss_tokens,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageRole {
    System,
    User,
    Assistant,
    Tool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "role")]
pub enum Message {
    #[serde(rename = "system")]
    System { content: String },
    #[serde(rename = "user")]
    User { content: String },
    #[serde(rename = "assistant")]
    Assistant {
        content: String,
        /// DeepSeek 推理模式的推理内容（如 deepseek-reasoner）
        #[serde(skip_serializing_if = "Option::is_none")]
        reasoning_content: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        tool_calls: Option<Vec<ToolCall>>,
    },
    #[serde(rename = "tool")]
    Tool {
        tool_call_id: String,
        content: String,
    },
}

impl Message {
    pub fn system(content: impl Into<String>) -> Self {
        Self::System {
            content: content.into(),
        }
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self::User {
            content: content.into(),
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self::Assistant {
            content: content.into(),
            reasoning_content: None,
            tool_calls: None,
        }
    }

    /// 创建带推理内容的 Assistant 消息
    pub fn assistant_with_reasoning(
        content: impl Into<String>,
        reasoning_content: impl Into<String>,
    ) -> Self {
        Self::Assistant {
            content: content.into(),
            reasoning_content: Some(reasoning_content.into()),
            tool_calls: None,
        }
    }

    /// 获取消息内容
    pub fn content(&self) -> &str {
        match self {
            Self::System { content } => content,
            Self::User { content } => content,
            Self::Assistant { content, .. } => content,
            Self::Tool { content, .. } => content,
        }
    }

    /// 获取推理内容（仅 Assistant 消息有）
    pub fn reasoning_content(&self) -> Option<&str> {
        match self {
            Self::Assistant {
                reasoning_content, ..
            } => reasoning_content.as_deref(),
            _ => None,
        }
    }
}
