use async_trait::async_trait;
use futures::{StreamExt, stream::BoxStream};
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE, HeaderMap};
use std::collections::HashMap;
use std::time::Duration;

use super::{Provider, ProviderError, Request, Response, StreamResponse, parse_api_error};

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

fn drain_sse_frames(buffer: &mut String) -> Vec<String> {
    let mut frames = Vec::new();
    while let Some(idx) = buffer.find("\n\n") {
        let frame = buffer[..idx].to_string();
        let remaining = buffer[idx + 2..].to_string();
        *buffer = remaining;
        if !frame.trim().is_empty() {
            frames.push(frame);
        }
    }
    frames
}

fn parse_sse_frame(frame: &str) -> Option<StreamResponse> {
    let payload = frame
        .lines()
        .filter_map(|line| line.strip_prefix("data:").map(str::trim_start))
        .collect::<Vec<_>>()
        .join("\n");

    if payload.is_empty() || payload == "[DONE]" {
        return None;
    }

    serde_json::from_str::<StreamResponse>(&payload).ok()
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
            .scan(String::new(), |buffer, chunk_result| {
                let chunk = match chunk_result {
                    Ok(c) => c,
                    Err(_) => return futures::future::ready(Some(vec![])),
                };

                buffer.push_str(&String::from_utf8_lossy(&chunk));
                if buffer.contains("\r\n") {
                    *buffer = buffer.replace("\r\n", "\n");
                }

                let parsed = drain_sse_frames(buffer)
                    .into_iter()
                    .filter_map(|frame| parse_sse_frame(&frame))
                    .collect::<Vec<_>>();

                futures::future::ready(Some(parsed))
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
    use serde_json::json;

    #[test]
    fn test_provider_creation() {
        let provider =
            OpenAICompatibleProvider::new("test", "test-api-key", "https://api.example.com");

        assert_eq!(provider.name(), "test");
        assert_eq!(provider.api_key, "test-api-key");
        assert_eq!(provider.base_url, "https://api.example.com");
    }

    #[test]
    fn test_with_extra_headers() {
        let mut extra = HashMap::new();
        extra.insert("X-Custom-Header".to_string(), "custom-value".to_string());

        let provider =
            OpenAICompatibleProvider::new("test", "test-api-key", "https://api.example.com")
                .with_extra_headers(extra);

        assert_eq!(provider.extra_headers.len(), 1);
        assert_eq!(
            provider.extra_headers.get("X-Custom-Header"),
            Some(&"custom-value".to_string())
        );
    }

    #[test]
    fn test_with_timeout() {
        let provider =
            OpenAICompatibleProvider::new("test", "test-api-key", "https://api.example.com")
                .with_timeout(Duration::from_secs(60));

        // 验证 provider 创建成功
        assert_eq!(provider.name(), "test");
    }

    #[test]
    fn test_drain_sse_frames_handles_split_chunks() {
        let mut buffer = String::new();
        buffer.push_str("data: {\"id\":\"abc\"");
        assert!(drain_sse_frames(&mut buffer).is_empty());

        buffer.push_str(",\"object\":\"chat.completion.chunk\",\"created\":1,\"model\":\"m\",\"system_fingerprint\":null,\"choices\":[],\"usage\":null}\n\n");
        let frames = drain_sse_frames(&mut buffer);

        assert_eq!(frames.len(), 1);
        assert!(buffer.is_empty());
    }

    #[test]
    fn test_parse_sse_frame_ignores_done_and_parses_json() {
        assert!(parse_sse_frame("data: [DONE]").is_none());

        let frame = format!(
            "data: {}\n",
            json!({
                "id": "abc",
                "object": "chat.completion.chunk",
                "created": 1,
                "model": "test-model",
                "system_fingerprint": null,
                "choices": [{
                    "index": 0,
                    "delta": { "content": "hi" },
                    "finish_reason": "stop"
                }],
                "usage": null
            })
        );

        let parsed = parse_sse_frame(&frame).expect("frame should parse");
        assert_eq!(parsed.choices.len(), 1);
        assert_eq!(parsed.choices[0].delta.content.as_deref(), Some("hi"));
        assert_eq!(parsed.choices[0].finish_reason.as_deref(), Some("stop"));
    }
}
