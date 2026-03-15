use async_trait::async_trait;
use futures::{StreamExt, stream::BoxStream};
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE, HeaderMap};
use std::collections::HashMap;
use std::time::Duration;

use super::{parse_api_error, parse_sse_line, Provider, ProviderError, Request, Response, StreamResponse};

/// OpenAI 兼容的 Provider 实现
///
/// 这是一个通用的 HTTP provider，可以用于任何兼容 OpenAI API 格式的服务。
/// 包括 DeepSeek、OpenRouter、Groq 等提供商。
pub struct OpenAICompatibleProvider {
    client: reqwest::Client,
    api_key: String,
    base_url: String,
    extra_headers: HashMap<String, String>,
    name: String,
}

impl OpenAICompatibleProvider {
    /// 创建新的 OpenAI 兼容 Provider
    ///
    /// # Arguments
    /// * `name` - Provider 名称（用于日志和调试）
    /// * `api_key` - API 密钥
    /// * `base_url` - API 基础 URL（如 "https://api.deepseek.com"）
    pub fn new(
        name: impl Into<String>,
        api_key: impl Into<String>,
        base_url: impl Into<String>,
    ) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(120))
            .build()
            .expect("Failed to build HTTP client");

        Self {
            client,
            api_key: api_key.into(),
            base_url: base_url.into(),
            extra_headers: HashMap::new(),
            name: name.into(),
        }
    }

    /// 添加额外的请求头
    pub fn with_extra_headers(mut self, headers: HashMap<String, String>) -> Self {
        self.extra_headers = headers;
        self
    }

    /// 设置请求超时时间
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.client = reqwest::Client::builder()
            .timeout(timeout)
            .build()
            .expect("Failed to build HTTP client");
        self
    }

    /// 设置自定义基础 URL
    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into();
        self
    }

    /// 构建请求头
    fn build_headers(&self) -> HeaderMap {
        use reqwest::header::HeaderName;

        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            format!("Bearer {}", self.api_key)
                .parse()
                .expect("Invalid API key format"),
        );
        headers.insert(CONTENT_TYPE, "application/json".parse().unwrap());

        // 添加额外的请求头
        for (key, value) in &self.extra_headers {
            if let Ok(name) = HeaderName::try_from(key.as_str())
                && let Ok(val) = value.parse()
            {
                headers.insert(name, val);
            }
        }

        headers
    }
}

#[async_trait]
impl Provider for OpenAICompatibleProvider {
    /// 发送非流式请求
    async fn chat(&self, request: Request) -> Result<Response, ProviderError> {
        let url = format!("{}/chat/completions", self.base_url);
        let headers = self.build_headers();

        let response = self
            .client
            .post(&url)
            .headers(headers)
            .json(&request)
            .send()
            .await?;

        let status = response.status();
        let body = response.text().await?;

        if !status.is_success() {
            return Err(parse_api_error(&body, status.as_u16()));
        }

        let response: Response = serde_json::from_str(&body)?;
        Ok(response)
    }

    /// 发送流式请求
    async fn chat_stream(
        &self,
        request: Request,
    ) -> Result<BoxStream<'static, StreamResponse>, ProviderError> {
        let url = format!("{}/chat/completions", self.base_url);
        let headers = self.build_headers();

        let mut stream_request = request;
        stream_request.stream = Some(true);

        let response = self
            .client
            .post(&url)
            .headers(headers)
            .json(&stream_request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(parse_api_error(&body, status.as_u16()));
        }

        let stream = response
            .bytes_stream()
            .map(|chunk_result| {
                let chunk = match chunk_result {
                    Ok(c) => c,
                    Err(_) => return vec![],
                };

                let text = String::from_utf8_lossy(&chunk).to_string();
                text.lines()
                    .filter_map(|line| {
                        parse_sse_line(line)
                            .and_then(|json| serde_json::from_value::<StreamResponse>(json).ok())
                    })
                    .collect::<Vec<_>>()
            })
            .flat_map(futures::stream::iter);

        Ok(Box::pin(stream))
    }

    /// Provider 名称
    fn name(&self) -> &str {
        &self.name
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_creation() {
        let provider = OpenAICompatibleProvider::new(
            "test",
            "test-api-key",
            "https://api.example.com",
        );

        assert_eq!(provider.name(), "test");
        assert_eq!(provider.api_key, "test-api-key");
        assert_eq!(provider.base_url, "https://api.example.com");
    }

    #[test]
    fn test_with_extra_headers() {
        let mut extra = HashMap::new();
        extra.insert("X-Custom-Header".to_string(), "custom-value".to_string());

        let provider = OpenAICompatibleProvider::new(
            "test",
            "test-api-key",
            "https://api.example.com",
        )
        .with_extra_headers(extra);

        assert_eq!(provider.extra_headers.len(), 1);
        assert_eq!(
            provider.extra_headers.get("X-Custom-Header"),
            Some(&"custom-value".to_string())
        );
    }

    #[test]
    fn test_with_timeout() {
        let provider = OpenAICompatibleProvider::new(
            "test",
            "test-api-key",
            "https://api.example.com",
        )
        .with_timeout(Duration::from_secs(60));

        // 验证 provider 创建成功
        assert_eq!(provider.name(), "test");
    }
}