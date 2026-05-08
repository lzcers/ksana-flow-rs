use crate::agent::{ToolCall, ToolDef};
use crate::core::MessageRole;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::HashMap;

use async_trait::async_trait;
use futures::stream::BoxStream;

/// Provider trait - LLM API 提供商的统一接口
#[async_trait]
pub trait Provider: Send + Sync {
    /// 发送非流式请求
    async fn chat(&self, request: Request) -> Result<Response, ProviderError>;

    /// 发送流式请求
    async fn chat_stream(
        &self,
        request: Request,
    ) -> Result<BoxStream<'static, StreamResponse>, ProviderError>;

    /// Provider 名称（用于日志和调试）
    fn name(&self) -> &str;
}

// ============================================================================
// Token 使用统计
// ============================================================================

/// Token 使用统计
/// 兼容 OpenAI 和 DeepSeek 的格式（prompt_tokens/input_tokens, completion_tokens/output_tokens）
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Usage {
    #[serde(alias = "input_tokens")]
    pub prompt_tokens: u32,
    #[serde(alias = "output_tokens")]
    pub completion_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_cache_hit_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_cache_miss_tokens: Option<u32>,
}

impl Usage {
    pub fn total(&self) -> u32 {
        self.total_tokens
            .unwrap_or(self.prompt_tokens + self.completion_tokens)
    }
}

// ============================================================================
// 错误类型
// ============================================================================

#[derive(Debug)]
pub enum ProviderError {
    Request(reqwest::Error),
    Serialization(serde_json::Error),
    InvalidApiKey,
    ApiError { code: u16, message: String },
    MissingApiKey,
    StreamError(String),
}

impl std::error::Error for ProviderError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ProviderError::Request(e) => Some(e),
            ProviderError::Serialization(e) => Some(e),
            _ => None,
        }
    }
}

impl std::fmt::Display for ProviderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProviderError::Request(e) => write!(f, "Request error: {}", e),
            ProviderError::Serialization(e) => write!(f, "Serialization error: {}", e),
            ProviderError::InvalidApiKey => write!(f, "Invalid API key"),
            ProviderError::ApiError { code, message } => {
                write!(f, "API error {}: {}", code, message)
            }
            ProviderError::MissingApiKey => write!(f, "Missing API key"),
            ProviderError::StreamError(msg) => write!(f, "Stream error: {}", msg),
        }
    }
}

impl From<reqwest::Error> for ProviderError {
    fn from(e: reqwest::Error) -> Self {
        ProviderError::Request(e)
    }
}

impl From<serde_json::Error> for ProviderError {
    fn from(e: serde_json::Error) -> Self {
        ProviderError::Serialization(e)
    }
}

// ============================================================================
// 请求参数
// ============================================================================

/// 请求参数
/// 基于 OpenAI 兼容格式，可扩展支持不同供应商的特有参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Request {
    /// 模型名称
    pub model: String,
    /// 消息列表
    pub messages: Vec<crate::core::Message>,
    /// 是否使用流式输出
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    /// 控制随机性 (0-2)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    /// 最大 token 数
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    /// 扩展字段，用于供应商特有参数
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

impl Request {
    pub fn new(model: impl Into<String>, messages: Vec<crate::core::Message>) -> Self {
        Self {
            model: model.into(),
            messages,
            stream: None,
            temperature: None,
            max_tokens: None,
            extra: HashMap::new(),
        }
    }

    pub fn with_stream(mut self, stream: bool) -> Self {
        self.stream = Some(stream);
        self
    }

    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = Some(temperature);
        self
    }

    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }

    pub fn with_stream_usage(mut self, include_usage: bool) -> Self {
        let stream_options = self
            .extra
            .entry("stream_options".to_string())
            .or_insert_with(|| json!({}));

        if let Some(options) = stream_options.as_object_mut() {
            options.insert("include_usage".to_string(), json!(include_usage));
        } else {
            *stream_options = json!({ "include_usage": include_usage });
        }

        self
    }

    pub fn with_tools(mut self, tools: Option<Vec<ToolDef>>) -> Self {
        if let Some(tools) = tools {
            let tools: Vec<Value> = tools
                .iter()
                .map(|def| {
                    json!({
                        "type": "function",
                        "function": {
                            "name": def.name,
                            "description": def.description,
                            "parameters": def.parameters,
                        }
                    })
                })
                .collect();
            self.extra.insert("tools".to_string(), json!(tools));
        }
        self
    }
    pub fn with_response_format_json(mut self) -> Self {
        self.extra.insert(
            "response_format".to_string(),
            json!({
                "type": "json_object",
            }),
        );
        self
    }

    pub fn with_reasoning_effort(mut self, reasoning_effort: impl Into<String>) -> Self {
        self.extra.insert(
            "reasoning_effort".to_string(),
            json!(reasoning_effort.into()),
        );
        self
    }

    pub fn with_thinking(mut self, enabled: bool) -> Self {
        self.extra.insert(
            "thinking".to_string(),
            json!({
                "type": if enabled { "enabled" } else { "disabled" },
            }),
        );
        self
    }
}

// ============================================================================
// 响应类型
// ============================================================================

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ChoiceImgUrl {
    pub url: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ChoiceImg {
    #[serde(rename = "type")]
    pub img_type: String,
    pub image_url: ChoiceImgUrl,
}

/// 选择项中的消息（非流式响应）
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ChoiceMessage {
    pub role: MessageRole,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    /// DeepSeek 推理模式的推理内容（如 deepseek-reasoner）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub images: Option<Vec<ChoiceImg>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

/// 非流式响应的选择项
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Choice {
    pub index: u32,
    pub message: ChoiceMessage,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<String>,
}

/// 非流式完整响应
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Response {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub choices: Vec<Choice>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<Usage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_fingerprint: Option<String>,
}

// ============================================================================
// 流式响应类型
// ============================================================================

/// 流式响应中的 delta 内容
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct Delta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<MessageRole>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    /// DeepSeek 推理模式的推理内容（如 deepseek-reasoner）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
}

/// 流式响应的选择项
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StreamChoice {
    pub index: u32,
    pub delta: Delta,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logprobs: Option<Value>,
}

/// 流式响应块
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StreamResponse {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub system_fingerprint: Option<String>,
    pub choices: Vec<StreamChoice>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<Usage>,
}

#[cfg(test)]
mod tests {
    use super::{Request, Response};
    use crate::core::Message;
    use serde_json::json;

    #[test]
    fn test_response_allows_null_content_for_images() {
        let body = r#"{
            "id": "resp_123",
            "object": "chat.completion",
            "created": 1743916800,
            "model": "black-forest-labs/flux.2-klein-4b",
            "choices": [
                {
                    "index": 0,
                    "message": {
                        "role": "assistant",
                        "content": null,
                        "images": [
                            {
                                "type": "image_url",
                                "image_url": {
                                    "url": "https://example.com/image.png"
                                }
                            }
                        ]
                    },
                    "finish_reason": "stop"
                }
            ]
        }"#;

        let response: Response = serde_json::from_str(body).unwrap();
        let choice = response.choices.into_iter().next().unwrap();

        assert_eq!(choice.message.content, None);
        assert_eq!(
            choice.message.images.unwrap()[0].image_url.url,
            "https://example.com/image.png"
        );
    }

    #[test]
    fn test_request_supports_reasoning_effort_and_thinking() {
        let request = Request::new("deepseek-v4-pro", vec![Message::user("hello")])
            .with_reasoning_effort("high")
            .with_thinking(true);

        assert_eq!(request.extra.get("reasoning_effort"), Some(&json!("high")));
        assert_eq!(
            request.extra.get("thinking"),
            Some(&json!({ "type": "enabled" }))
        );
    }
}
