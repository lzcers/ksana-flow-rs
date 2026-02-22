use super::{Provider, ProviderError, Request, Response, StreamResponse};
use futures::{Stream, StreamExt};
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE, HeaderMap};
use std::env;

/// DeepSeek API 提供商
/// 支持从环境变量 `DEEPSEEK_API_KEY` 获取 API 密钥
pub struct DeepSeekProvider {
    client: reqwest::Client,
    api_key: String,
    base_url: String,
}

impl DeepSeekProvider {
    /// 从环境变量创建 DeepSeekProvider
    /// 环境变量: DEEPSEEK_API_KEY
    pub fn from_env() -> Result<Self, ProviderError> {
        let api_key = env::var("DEEPSEEK_API_KEY").map_err(|_| ProviderError::MissingApiKey)?;

        Ok(Self::new(api_key))
    }

    /// 使用指定的 API 密钥创建 DeepSeekProvider
    pub fn new(api_key: impl Into<String>) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .expect("Failed to build HTTP client");

        Self {
            client,
            api_key: api_key.into(),
            base_url: "https://api.deepseek.com".to_string(),
        }
    }

    /// 设置自定义基础 URL（用于测试或其他端点）
    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into();
        self
    }

    fn build_headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            format!("Bearer {}", self.api_key)
                .parse()
                .expect("Invalid API key format"),
        );
        headers.insert(CONTENT_TYPE, "application/json".parse().unwrap());
        headers
    }

    /// 解析 SSE 格式的流数据
    pub(crate) fn parse_sse_line(line: &str) -> Option<serde_json::Value> {
        let line = line.trim();
        if line.is_empty() || line == "data: [DONE]" {
            return None;
        }
        if let Some(data) = line.strip_prefix("data: ") {
            return serde_json::from_str(data).ok();
        }
        None
    }
}

impl Provider for DeepSeekProvider {
    async fn send_request(
        &self,
        path: &str,
        request: &Request,
        _model: &str,
    ) -> Result<Response, ProviderError> {
        let url = format!("{}{}", self.base_url, path);
        let headers = self.build_headers();

        let response = self
            .client
            .post(&url)
            .headers(headers)
            .json(request)
            .send()
            .await?;

        let status = response.status();
        let body = response.text().await?;

        if !status.is_success() {
            // 尝试解析错误响应
            if let Ok(error_json) = serde_json::from_str::<serde_json::Value>(&body) {
                let code = error_json["error"]["code"]
                    .as_i64()
                    .or_else(|| error_json["error"]["type"].as_str().map(|t| t.len() as i64))
                    .unwrap_or(0) as i32;
                let message = error_json["error"]["message"]
                    .as_str()
                    .or_else(|| error_json["error"].as_str())
                    .unwrap_or(&body)
                    .to_string();
                return Err(ProviderError::ApiError { code, message });
            }
            return Err(ProviderError::ApiError {
                code: status.as_u16() as i32,
                message: body,
            });
        }

        let response: Response = serde_json::from_str(&body)?;
        Ok(response)
    }

    async fn stream_request(
        &self,
        path: &str,
        mut request: Request,
        _model: &str,
    ) -> Result<impl Stream<Item = StreamResponse>, ProviderError> {
        let url = format!("{}{}", self.base_url, path);
        let headers = self.build_headers();

        request.stream = Some(true);

        let response = self
            .client
            .post(&url)
            .headers(headers)
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(ProviderError::ApiError {
                code: status.as_u16() as i32,
                message: body,
            });
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
                        Self::parse_sse_line(line)
                            .and_then(|json| serde_json::from_value::<StreamResponse>(json).ok())
                    })
                    .collect::<Vec<StreamResponse>>()
            })
            .flat_map(futures::stream::iter);

        Ok(stream)
    }

    fn http_client(&self) -> &reqwest::Client {
        &self.client
    }
}
