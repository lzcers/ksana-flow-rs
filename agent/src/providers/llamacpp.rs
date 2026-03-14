use super::{Provider, ProviderError, Request, Response, StreamResponse};
use async_trait::async_trait;
use futures::{StreamExt, stream::BoxStream};
use reqwest::header::{CONTENT_TYPE, HeaderMap};
use std::env;

/// llama.cpp 本地 API 提供商
/// 支持从环境变量 `LLAMACPP_BASE_URL` 获取服务器地址，默认为 `http://localhost:8001`
/// llama.cpp 服务器通常不需要 API 密钥
pub struct LlamaCppProvider {
    client: reqwest::Client,
    base_url: String,
}

impl LlamaCppProvider {
    /// 从环境变量创建 LlamaCppProvider
    /// 环境变量: LLAMACPP_BASE_URL (可选，默认 http://localhost:8001)
    pub fn from_env() -> Result<Self, ProviderError> {
        let base_url =
            env::var("LLAMACPP_BASE_URL").unwrap_or_else(|_| "http://localhost:8001".to_string());

        Ok(Self::new(base_url))
    }

    /// 使用指定的服务器地址创建 LlamaCppProvider
    pub fn new(base_url: impl Into<String>) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(300)) // 本地推理可能较慢
            .build()
            .expect("Failed to build HTTP client");

        Self {
            client,
            base_url: base_url.into(),
        }
    }

    /// 设置自定义基础 URL
    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into();
        self
    }

    fn build_headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();
        // llama.cpp 服务器通常不需要认证
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

#[async_trait]
impl Provider for LlamaCppProvider {
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
                    .unwrap_or(0);
                let message = error_json["error"]["message"]
                    .as_str()
                    .or_else(|| error_json["error"].as_str())
                    .unwrap_or(&body)
                    .to_string();
                return Err(ProviderError::ApiError {
                    code: code as u16,
                    message,
                });
            }
            return Err(ProviderError::ApiError {
                code: status.as_u16(),
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
    ) -> Result<BoxStream<'static, StreamResponse>, ProviderError> {
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
                code: status.as_u16(),
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

        Ok(Box::pin(stream))
    }

    fn http_client(&self) -> &reqwest::Client {
        &self.client
    }
}
