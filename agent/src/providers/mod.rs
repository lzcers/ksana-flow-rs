mod openai_compatible;

use crate::agents::{ToolCall, ToolDef};
use crate::core::{Message, MessageRole};
use async_trait::async_trait;
pub use openai_compatible::OpenAICompatibleProvider;
use futures::stream::BoxStream;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::env;

// ============================================================================
// 工具函数
// ============================================================================

/// 解析 SSE 格式的流数据行
///
/// SSE 格式为 "data: {json}" 或 "data: [DONE]"
pub fn parse_sse_line(line: &str) -> Option<Value> {
    let line = line.trim();
    if line.is_empty() || line == "data: [DONE]" {
        return None;
    }
    if let Some(data) = line.strip_prefix("data: ") {
        return serde_json::from_str(data).ok();
    }
    None
}

/// 解析 API 错误响应
///
/// 尝试从响应体中提取错误代码和消息
pub fn parse_api_error(body: &str, status: u16) -> ProviderError {
    if let Ok(error_json) = serde_json::from_str::<Value>(body) {
        let code = error_json["error"]["code"]
            .as_i64()
            .or_else(|| error_json["error"]["type"].as_str().map(|t| t.len() as i64))
            .unwrap_or(0) as u16;
        let message = error_json["error"]["message"]
            .as_str()
            .or_else(|| error_json["error"].as_str())
            .unwrap_or(body)
            .to_string();
        return ProviderError::ApiError { code, message };
    }
    ProviderError::ApiError {
        code: status,
        message: body.to_string(),
    }
}

// ============================================================================
// 工厂函数
// ============================================================================

/// 创建 DeepSeek provider
///
/// # Example
/// ```no_run
/// use agent::providers::deepseek_provider;
/// let provider = deepseek_provider("your-api-key");
/// ```
pub fn deepseek_provider(api_key: impl Into<String>) -> OpenAICompatibleProvider {
    let base_url = env::var("DEEPSEEK_BASE_URL")
        .unwrap_or_else(|_| "https://api.deepseek.com".to_string());
    OpenAICompatibleProvider::new("deepseek", api_key, base_url)
}

/// 从环境变量创建 DeepSeek provider
///
/// 环境变量: DEEPSEEK_API_KEY (必需), DEEPSEEK_BASE_URL (可选)
pub fn deepseek_provider_from_env() -> Result<OpenAICompatibleProvider, ProviderError> {
    let api_key = env::var("DEEPSEEK_API_KEY").map_err(|_| ProviderError::MissingApiKey)?;
    Ok(deepseek_provider(api_key))
}

/// 创建 OpenRouter provider
///
/// # Example
/// ```no_run
/// use agent::providers::openrouter_provider;
/// let provider = openrouter_provider("your-api-key");
/// ```
pub fn openrouter_provider(api_key: impl Into<String>) -> OpenAICompatibleProvider {
    let base_url = env::var("OPENROUTER_BASE_URL")
        .unwrap_or_else(|_| "https://openrouter.ai/api/v1".to_string());
    OpenAICompatibleProvider::new("openrouter", api_key, base_url)
}

/// 创建 OpenRouter provider（带额外配置）
///
/// # Arguments
/// * `api_key` - API 密钥
/// * `http_referer` - HTTP-Referer 请求头（可选）
/// * `x_title` - X-Title 请求头（可选）
pub fn openrouter_provider_with_config(
    api_key: impl Into<String>,
    http_referer: Option<String>,
    x_title: Option<String>,
) -> OpenAICompatibleProvider {
    let mut extra = HashMap::new();
    if let Some(r) = http_referer {
        extra.insert("HTTP-Referer".into(), r);
    }
    if let Some(t) = x_title {
        extra.insert("X-Title".into(), t);
    }

    let base_url = env::var("OPENROUTER_BASE_URL")
        .unwrap_or_else(|_| "https://openrouter.ai/api/v1".to_string());

    OpenAICompatibleProvider::new("openrouter", api_key, base_url)
        .with_extra_headers(extra)
}

/// 从环境变量创建 OpenRouter provider
///
/// 环境变量:
/// - OPENROUTER_API_KEY (必需)
/// - OPENROUTER_BASE_URL (可选)
/// - OPENROUTER_HTTP_REFERER (可选)
/// - OPENROUTER_X_TITLE (可选)
pub fn openrouter_provider_from_env() -> Result<OpenAICompatibleProvider, ProviderError> {
    let api_key = env::var("OPENROUTER_API_KEY").map_err(|_| ProviderError::MissingApiKey)?;
    let http_referer = env::var("OPENROUTER_HTTP_REFERER").ok();
    let x_title = env::var("OPENROUTER_X_TITLE").ok();
    Ok(openrouter_provider_with_config(api_key, http_referer, x_title))
}

// ============================================================================
// 兼容性别名
// ============================================================================

/// DeepSeek Provider (已弃用，请使用 `deepseek_provider()`)
#[deprecated(
    since = "0.2.0",
    note = "请使用 `deepseek_provider()` 或 `deepseek_provider_from_env()` 函数"
)]
pub type DeepSeekProvider = OpenAICompatibleProvider;

/// OpenRouter Provider (已弃用，请使用 `openrouter_provider()`)
#[deprecated(
    since = "0.2.0",
    note = "请使用 `openrouter_provider()` 或 `openrouter_provider_from_env()` 函数"
)]
pub type OpenRouterProvider = OpenAICompatibleProvider;

// ============================================================================
// 核心类型
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
}

impl Usage {
    pub fn total(&self) -> u32 {
        self.total_tokens
            .unwrap_or(self.prompt_tokens + self.completion_tokens)
    }
}

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

/// 请求参数
/// 基于 OpenAI 兼容格式，可扩展支持不同供应商的特有参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Request {
    /// 模型名称
    pub model: String,
    /// 消息列表
    pub messages: Vec<Message>,
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
    pub fn new(model: impl Into<String>, messages: Vec<Message>) -> Self {
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
}

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
    pub content: String,
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

// ============================================================================
// Provider Trait
// ============================================================================

/// Provider trait - LLM API 提供商的统一接口
#[async_trait]
pub trait Provider: Send + Sync {
    /// 发送非流式请求
    async fn chat(&self, request: Request) -> Result<Response, ProviderError>;

    /// 发送流式请求
    async fn chat_stream(&self, request: Request) -> Result<BoxStream<'static, StreamResponse>, ProviderError>;

    /// Provider 名称（用于日志和调试）
    fn name(&self) -> &str;
}