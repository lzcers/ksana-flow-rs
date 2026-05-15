use super::{OpenAICompatibleProvider, ProviderError};
use std::env;

/// 创建 DeepSeek provider
///
/// # Example
/// ```no_run
/// use agent::providers::deepseek_provider;
/// let provider = deepseek_provider("your-api-key");
/// ```
pub fn deepseek_provider(api_key: impl Into<String>) -> OpenAICompatibleProvider {
    let base_url =
        env::var("DEEPSEEK_BASE_URL").unwrap_or_else(|_| "https://api.deepseek.com".to_string());
    OpenAICompatibleProvider::new("deepseek", api_key, base_url)
}

/// 从环境变量创建 DeepSeek provider
///
/// 环境变量: DEEPSEEK_API_KEY (必需), DEEPSEEK_BASE_URL (可选)
pub fn deepseek_provider_from_env() -> Result<OpenAICompatibleProvider, ProviderError> {
    let api_key = env::var("DEEPSEEK_API_KEY").map_err(|_| ProviderError::MissingApiKey)?;
    Ok(deepseek_provider(api_key))
}

pub type DeepSeekProvider = OpenAICompatibleProvider;
