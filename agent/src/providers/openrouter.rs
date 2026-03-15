use super::{OpenAICompatibleProvider, ProviderError};
use std::collections::HashMap;
use std::env;

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

    OpenAICompatibleProvider::new("openrouter", api_key, base_url).with_extra_headers(extra)
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
    Ok(openrouter_provider_with_config(
        api_key,
        http_referer,
        x_title,
    ))
}

pub type OpenRouterProvider = OpenAICompatibleProvider;
