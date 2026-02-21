use super::super::core::{Content, Message, Role, Usage};
use super::utils::{content_to_value, role_to_string};
use super::{
    ImageGenerationProvider, ImageGenerationRequest, ImageGenerationResponse,
    ImageGenerationStreamResponse, ModelProviderError,
};
use async_trait::async_trait;
use reqwest::Client;
use serde_json::{Value, json};
use std::collections::HashMap;

const DEFAULT_BASE_URL: &str = "https://openrouter.ai/api/v1";

pub struct OpenRouterProvider {
    client: Client,
    api_key: String,
    base_url: String,
}

impl OpenRouterProvider {
    pub fn from_env() -> Result<Self, ModelProviderError> {
        let api_key = std::env::var("OPENROUTER_API_KEY")
            .map_err(|_| ModelProviderError::MissingEnvVar("OPENROUTER_API_KEY".to_string()))?;
        let base_url =
            std::env::var("OPENROUTER_BASE_URL").unwrap_or_else(|_| DEFAULT_BASE_URL.to_string());
        Ok(Self {
            client: Client::new(),
            api_key,
            base_url,
        })
    }

    pub fn new(api_key: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
            base_url: DEFAULT_BASE_URL.to_string(),
        }
    }

    pub fn with_base_url(api_key: String, base_url: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
            base_url,
        }
    }
}

fn map_message(message: &Message) -> Value {
    let mut map = serde_json::Map::new();
    map.insert("role".to_string(), json!(role_to_string(&message.role)));
    map.insert("content".to_string(), content_to_value(&message.content));
    if let Some(name) = &message.name {
        map.insert("name".to_string(), json!(name));
    }
    Value::Object(map)
}

fn build_payload(request: &ImageGenerationRequest) -> Value {
    let mut payload = serde_json::Map::new();
    let messages: Vec<Value> = request.messages.iter().map(map_message).collect();
    payload.insert("model".to_string(), json!(request.model));
    payload.insert("messages".to_string(), json!(messages));
    payload.insert("stream".to_string(), json!(request.options.stream));
    if let Some(n) = request.options.n {
        payload.insert("n".to_string(), json!(n));
    }
    if !request.extensions.contains_key("modalities") {
        payload.insert("modalities".to_string(), json!(["image"]));
    }
    if request.options.aspect_ratio.is_some() || request.options.image_size.is_some() {
        let mut image_config = serde_json::Map::new();
        if let Some(aspect_ratio) = &request.options.aspect_ratio {
            image_config.insert("aspect_ratio".to_string(), json!(aspect_ratio));
        }
        if let Some(image_size) = &request.options.image_size {
            image_config.insert("image_size".to_string(), json!(image_size));
        }
        payload.insert("image_config".to_string(), Value::Object(image_config));
    }
    for (key, value) in &request.extensions {
        payload.insert(key.clone(), value.clone());
    }
    Value::Object(payload)
}

fn parse_usage(response: &Value) -> Option<Usage> {
    let usage = response.get("usage")?;
    let prompt_tokens = usage
        .get("prompt_tokens")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u32;
    let completion_tokens = usage
        .get("completion_tokens")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u32;
    Some(Usage {
        input_tokens: prompt_tokens,
        output_tokens: completion_tokens,
    })
}

fn extract_images(response: &Value) -> Result<Vec<Content>, ModelProviderError> {
    let message = response
        .get("choices")
        .and_then(|v| v.get(0))
        .and_then(|v| v.get("message"))
        .ok_or_else(|| ModelProviderError::InvalidResponse("choices empty".to_string()))?;

    let mut images = Vec::new();
    if let Some(items) = message.get("images").and_then(|v| v.as_array()) {
        for item in items {
            if let Some(url) = item
                .get("image_url")
                .and_then(|v| v.get("url"))
                .and_then(|v| v.as_str())
            {
                images.push(Content::ImageUrl {
                    url: url.to_string(),
                    detail: None,
                });
            }
        }
    }

    match message.get("content") {
        Some(Value::String(s)) => {
            if s.trim_start().starts_with("data:image/") {
                images.push(Content::ImageUrl {
                    url: s.to_string(),
                    detail: None,
                });
            }
        }
        Some(Value::Array(parts)) => {
            for part in parts {
                if part.get("type").and_then(|v| v.as_str()) != Some("image_url") {
                    continue;
                }
                let image_url = part.get("image_url");
                let url = image_url
                    .and_then(|v| v.get("url"))
                    .and_then(|v| v.as_str());
                if let Some(url) = url {
                    let detail = image_url
                        .and_then(|v| v.get("detail"))
                        .and_then(|v| v.as_str())
                        .map(|v| v.to_string());
                    images.push(Content::ImageUrl {
                        url: url.to_string(),
                        detail,
                    });
                }
            }
        }
        _ => {}
    }

    if images.is_empty() {
        if let Some(err) = response.get("error") {
            return Err(ModelProviderError::InvalidResponse(err.to_string()));
        }
        return Err(ModelProviderError::InvalidResponse(
            "No image returned from model".to_string(),
        ));
    }
    Ok(images)
}

#[async_trait]
impl ImageGenerationProvider for OpenRouterProvider {
    fn name(&self) -> &str {
        "openrouter"
    }

    async fn generate(
        &self,
        request: ImageGenerationRequest,
    ) -> Result<ImageGenerationResponse, ModelProviderError> {
        let payload = build_payload(&request);
        let url = format!("{}/chat/completions", self.base_url.trim_end_matches('/'));
        let mut req = self
            .client
            .post(url)
            .bearer_auth(&self.api_key)
            .json(&payload);

        if let Ok(referer) = std::env::var("OPENROUTER_HTTP_REFERER") {
            req = req.header("HTTP-Referer", referer);
        }
        if let Ok(title) = std::env::var("OPENROUTER_X_TITLE") {
            req = req.header("X-Title", title);
        }

        let response = req
            .send()
            .await
            .map_err(|e| ModelProviderError::RequestFailed(e.to_string()))?;

        let status = response.status();
        let resp_json: Value = response
            .json()
            .await
            .map_err(|e| ModelProviderError::ResponseParseFailed(e.to_string()))?;

        if !status.is_success() {
            return Err(ModelProviderError::ApiError {
                status: status.as_u16(),
                body: resp_json.to_string(),
            });
        }

        let id = resp_json
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        let model = resp_json
            .get("model")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        let finish_reason = resp_json
            .get("choices")
            .and_then(|v| v.get(0))
            .and_then(|v| v.get("finish_reason"))
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();

        let images = extract_images(&resp_json)?;
        let usage = parse_usage(&resp_json);
        let mut extensions = HashMap::new();
        extensions.insert("raw".to_string(), resp_json);

        Ok(ImageGenerationResponse {
            id,
            model,
            images,
            usage,
            finish_reason,
            extensions,
        })
    }

    async fn generate_stream(
        &self,
        _request: ImageGenerationRequest,
    ) -> Result<ImageGenerationStreamResponse, ModelProviderError> {
        Err(ModelProviderError::StreamNotSupported)
    }
}

#[cfg(test)]
mod tests {
    use super::super::ImageGenerationOptions;
    use super::super::ImageGenerationRequest;
    use super::*;

    #[test]
    fn build_payload_text_only() {
        let request = ImageGenerationRequest {
            model: "test-model".to_string(),
            messages: vec![Message {
                role: Role::User,
                content: vec![Content::Text("hello".to_string())],
                name: None,
            }],
            options: ImageGenerationOptions {
                aspect_ratio: Some("1:1".to_string()),
                image_size: Some("1024x1024".to_string()),
                n: Some(1),
                stream: false,
            },
            extensions: HashMap::new(),
        };
        let payload = build_payload(&request);
        assert_eq!(payload.get("model").unwrap(), "test-model");
        assert_eq!(
            payload
                .get("messages")
                .and_then(|v| v.get(0))
                .and_then(|v| v.get("content"))
                .unwrap(),
            "hello"
        );
        assert_eq!(
            payload
                .get("image_config")
                .and_then(|v| v.get("aspect_ratio"))
                .unwrap(),
            "1:1"
        );
        assert_eq!(
            payload
                .get("image_config")
                .and_then(|v| v.get("image_size"))
                .unwrap(),
            "1024x1024"
        );
    }

    #[test]
    fn build_payload_with_image() {
        let request = ImageGenerationRequest {
            model: "test-model".to_string(),
            messages: vec![Message {
                role: Role::User,
                content: vec![
                    Content::ImageUrl {
                        url: "data:image/png;base64,aaa".to_string(),
                        detail: None,
                    },
                    Content::Text("prompt".to_string()),
                ],
                name: None,
            }],
            options: ImageGenerationOptions {
                aspect_ratio: None,
                image_size: None,
                n: None,
                stream: false,
            },
            extensions: HashMap::new(),
        };
        let payload = build_payload(&request);
        let content = payload
            .get("messages")
            .and_then(|v| v.get(0))
            .and_then(|v| v.get("content"))
            .and_then(|v| v.as_array())
            .unwrap();
        assert_eq!(content.len(), 2);
        assert_eq!(
            content.get(0).and_then(|v| v.get("type")).unwrap(),
            "image_url"
        );
        assert_eq!(content.get(1).and_then(|v| v.get("type")).unwrap(), "text");
    }

    #[test]
    fn extract_images_from_response() {
        let response = json!({
            "id": "resp-1",
            "model": "test",
            "choices": [{
                "message": {
                    "images": [{
                        "image_url": { "url": "data:image/png;base64,abc" }
                    }]
                },
                "finish_reason": "stop"
            }],
            "usage": { "prompt_tokens": 10, "completion_tokens": 20 }
        });
        let images = extract_images(&response).unwrap();
        assert_eq!(images.len(), 1);
        match &images[0] {
            Content::ImageUrl { url, .. } => assert_eq!(url, "data:image/png;base64,abc"),
            _ => panic!("unexpected content"),
        }
    }
}
