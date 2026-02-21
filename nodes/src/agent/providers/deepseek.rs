use super::super::core::{Content, Message, Usage};
use super::utils::{content_to_text, role_to_string};
use super::{
    CompletionProvider, CompletionRequest, CompletionResponse, CompletionStreamChunk,
    CompletionStreamResponse, ModelProviderError,
};
use async_trait::async_trait;
use bytes::Bytes;
use futures::{Stream, StreamExt};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{HashMap, VecDeque};
use std::pin::Pin;

const DEFAULT_BASE_URL: &str = "https://api.deepseek.com/chat/completions";

pub struct DeepSeekProvider {
    client: Client,
    api_key: String,
    base_url: String,
}

impl DeepSeekProvider {
    pub fn from_env() -> Result<Self, ModelProviderError> {
        let api_key = std::env::var("DEEPSEEK_API_KEY")
            .map_err(|_| ModelProviderError::MissingEnvVar("DEEPSEEK_API_KEY".to_string()))?;
        let base_url =
            std::env::var("DEEPSEEK_BASE_URL").unwrap_or_else(|_| DEFAULT_BASE_URL.to_string());
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

#[derive(Serialize)]
struct DeepSeekRequestMessage {
    role: String,
    content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
}

#[derive(Deserialize)]
struct DeepSeekResponse {
    id: String,
    model: String,
    choices: Vec<DeepSeekChoice>,
    usage: Option<DeepSeekUsage>,
}

#[derive(Deserialize)]
struct DeepSeekChoice {
    message: DeepSeekResponseMessage,
    finish_reason: Option<String>,
}

#[derive(Deserialize)]
struct DeepSeekResponseMessage {
    content: String,
}

#[derive(Deserialize)]
struct DeepSeekStreamResponse {
    id: String,
    model: String,
    choices: Vec<DeepSeekStreamChoice>,
}

#[derive(Deserialize)]
struct DeepSeekStreamChoice {
    delta: DeepSeekStreamDelta,
    finish_reason: Option<String>,
}

#[derive(Deserialize)]
struct DeepSeekStreamDelta {
    role: Option<String>,
    content: Option<String>,
}

#[derive(Deserialize)]
struct DeepSeekUsage {
    prompt_tokens: Option<u32>,
    completion_tokens: Option<u32>,
}

fn map_message(message: &Message) -> DeepSeekRequestMessage {
    DeepSeekRequestMessage {
        role: role_to_string(&message.role),
        content: content_to_text(&message.content),
        name: message.name.clone(),
    }
}

fn build_payload(request: &CompletionRequest, stream: bool) -> Value {
    let mut payload = serde_json::Map::new();
    let messages: Vec<DeepSeekRequestMessage> = request.messages.iter().map(map_message).collect();
    payload.insert("model".to_string(), json!(request.model));
    payload.insert("messages".to_string(), json!(messages));
    payload.insert("stream".to_string(), json!(stream));

    if let Some(temperature) = request.options.temperature {
        payload.insert("temperature".to_string(), json!(temperature));
    }
    if let Some(max_tokens) = request.options.max_tokens {
        payload.insert("max_tokens".to_string(), json!(max_tokens));
    }
    if let Some(top_p) = request.options.top_p {
        payload.insert("top_p".to_string(), json!(top_p));
    }

    for (key, value) in &request.extensions {
        payload.insert(key.clone(), value.clone());
    }

    Value::Object(payload)
}

enum StreamParseOutcome {
    Chunk(CompletionStreamChunk),
    Done,
    Ignore,
}

fn parse_sse_line(line: &str) -> Result<StreamParseOutcome, ModelProviderError> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return Ok(StreamParseOutcome::Ignore);
    }
    let data = match trimmed.strip_prefix("data:") {
        Some(value) => value.trim(),
        None => return Ok(StreamParseOutcome::Ignore),
    };
    if data == "[DONE]" {
        return Ok(StreamParseOutcome::Done);
    }
    let value: Value = serde_json::from_str(data)
        .map_err(|e| ModelProviderError::ResponseParseFailed(e.to_string()))?;
    let parsed: DeepSeekStreamResponse = serde_json::from_value(value.clone())
        .map_err(|e| ModelProviderError::ResponseParseFailed(e.to_string()))?;
    let choice = parsed
        .choices
        .first()
        .ok_or_else(|| ModelProviderError::InvalidResponse("choices empty".to_string()))?;
    let text = choice.delta.content.clone().unwrap_or_default();
    let content = if text.is_empty() {
        Vec::new()
    } else {
        vec![Content::Text(text)]
    };
    let mut extensions = HashMap::new();
    extensions.insert("raw".to_string(), value);
    if let Some(role) = &choice.delta.role {
        extensions.insert("role".to_string(), json!(role));
    }
    Ok(StreamParseOutcome::Chunk(CompletionStreamChunk {
        id: parsed.id,
        model: parsed.model,
        content,
        finish_reason: choice.finish_reason.clone(),
        extensions,
    }))
}

struct DeepSeekStreamState {
    stream: Pin<Box<dyn Stream<Item = Result<Bytes, reqwest::Error>> + Send>>,
    buffer: String,
    pending: VecDeque<CompletionStreamChunk>,
    done: bool,
}

#[async_trait]
impl CompletionProvider for DeepSeekProvider {
    fn name(&self) -> &str {
        "deepseek"
    }

    async fn complete(
        &self,
        request: CompletionRequest,
    ) -> Result<CompletionResponse, ModelProviderError> {
        let payload = build_payload(&request, false);
        let response = self
            .client
            .post(&self.base_url)
            .bearer_auth(&self.api_key)
            .json(&payload)
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

        let parsed: DeepSeekResponse = serde_json::from_value(resp_json.clone())
            .map_err(|e| ModelProviderError::ResponseParseFailed(e.to_string()))?;

        let first_choice = parsed
            .choices
            .first()
            .ok_or_else(|| ModelProviderError::InvalidResponse("choices empty".to_string()))?;

        let content = vec![Content::Text(first_choice.message.content.clone())];
        let finish_reason = first_choice.finish_reason.clone().unwrap_or_default();
        let usage = parsed.usage.map(|usage| Usage {
            input_tokens: usage.prompt_tokens.unwrap_or(0),
            output_tokens: usage.completion_tokens.unwrap_or(0),
        });

        let mut extensions = HashMap::new();
        extensions.insert("raw".to_string(), resp_json);

        Ok(CompletionResponse {
            id: parsed.id,
            model: parsed.model,
            content,
            usage,
            finish_reason,
            extensions,
        })
    }

    async fn complete_stream(
        &self,
        request: CompletionRequest,
    ) -> Result<CompletionStreamResponse, ModelProviderError> {
        let payload = build_payload(&request, true);
        let response = self
            .client
            .post(&self.base_url)
            .bearer_auth(&self.api_key)
            .json(&payload)
            .send()
            .await
            .map_err(|e| ModelProviderError::RequestFailed(e.to_string()))?;

        let status = response.status();
        if !status.is_success() {
            let body = response
                .text()
                .await
                .map_err(|e| ModelProviderError::ResponseParseFailed(e.to_string()))?;
            return Err(ModelProviderError::ApiError {
                status: status.as_u16(),
                body,
            });
        }

        let bytes_stream = response.bytes_stream();
        let state = DeepSeekStreamState {
            stream: Box::pin(bytes_stream),
            buffer: String::new(),
            pending: VecDeque::new(),
            done: false,
        };
        let stream = futures::stream::unfold(state, |mut state| async move {
            if state.done {
                return None;
            }
            loop {
                if let Some(chunk) = state.pending.pop_front() {
                    return Some((Ok(chunk), state));
                }
                match state.stream.next().await {
                    Some(Ok(bytes)) => {
                        let piece = String::from_utf8_lossy(&bytes);
                        state.buffer.push_str(&piece);
                        while let Some(pos) = state.buffer.find('\n') {
                            let line: String = state.buffer.drain(..=pos).collect();
                            let line = line.trim_end_matches('\n').trim_end_matches('\r');
                            match parse_sse_line(line) {
                                Ok(StreamParseOutcome::Chunk(chunk)) => {
                                    state.pending.push_back(chunk);
                                }
                                Ok(StreamParseOutcome::Done) => {
                                    state.done = true;
                                    break;
                                }
                                Ok(StreamParseOutcome::Ignore) => {}
                                Err(err) => return Some((Err(err), state)),
                            }
                        }
                        if state.done {
                            return None;
                        }
                    }
                    Some(Err(err)) => {
                        return Some((
                            Err(ModelProviderError::RequestFailed(err.to_string())),
                            state,
                        ));
                    }
                    None => return None,
                }
            }
        })
        .boxed();

        Ok(CompletionStreamResponse { stream })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::core::{Content, Message, Role};
    use crate::agent::providers::{CompletionOptions, CompletionProvider, CompletionRequest};
    use std::collections::HashMap;

    #[test]
    fn parse_stream_line_with_content() {
        let line = r#"data: {"id":"47d7bb1c-cae9-419a-b5e4-fd705ff2002e","model":"deepseek-chat","choices":[{"delta":{"role":"assistant","content":"Hello!"},"finish_reason":null}]}"#;
        let result = parse_sse_line(line).unwrap();
        match result {
            StreamParseOutcome::Chunk(chunk) => {
                assert_eq!(chunk.id, "47d7bb1c-cae9-419a-b5e4-fd705ff2002e");
                assert_eq!(chunk.model, "deepseek-chat");
                assert_eq!(chunk.finish_reason, None);
                assert_eq!(chunk.content.len(), 1);
                match &chunk.content[0] {
                    Content::Text(value) => assert_eq!(value, "Hello!"),
                    Content::ImageUrl { .. } => panic!("unexpected image content"),
                }
            }
            _ => panic!("expected chunk"),
        }
    }

    #[test]
    fn parse_stream_line_empty_content() {
        let line = r#"data: {"id":"47d7bb1c-cae9-419a-b5e4-fd705ff2002e","model":"deepseek-chat","choices":[{"delta":{"role":"assistant","content":""},"finish_reason":null}]}"#;
        let result = parse_sse_line(line).unwrap();
        match result {
            StreamParseOutcome::Chunk(chunk) => {
                assert_eq!(chunk.content.len(), 0);
            }
            _ => panic!("expected chunk"),
        }
    }

    #[test]
    fn parse_stream_line_done() {
        let line = "data: [DONE]";
        let result = parse_sse_line(line).unwrap();
        match result {
            StreamParseOutcome::Done => {}
            _ => panic!("expected done"),
        }
    }

    #[tokio::test]
    async fn deepseek_complete_from_env() {
        dotenv::dotenv().ok();
        if std::env::var("DEEPSEEK_API_KEY").is_err() {
            println!("skip deepseek_complete_from_env: miss DEEPSEEK_API_KEY");
            return;
        }
        let provider = DeepSeekProvider::from_env().unwrap();
        let request = CompletionRequest {
            model: "deepseek-chat".to_string(),
            messages: vec![Message {
                role: Role::User,
                content: vec![Content::Text("你好".to_string())],
                name: None,
            }],
            options: CompletionOptions {
                temperature: Some(0.0),
                max_tokens: None,
                top_p: None,
                stream: false,
            },
            extensions: HashMap::new(),
        };
        let response = provider.complete(request).await.unwrap();
        let mut text = String::new();
        for item in &response.content {
            match item {
                Content::Text(value) => text.push_str(value),
                Content::ImageUrl { .. } => {}
            }
        }
        println!("{}", text);
        assert!(!response.content.is_empty());
    }

    #[tokio::test]
    async fn deepseek_complete_stream_from_env() {
        dotenv::dotenv().ok();
        if std::env::var("DEEPSEEK_API_KEY").is_err() {
            println!(
                "skip deepseek_complete_stream_from_env: missing RUN_EXTERNAL_TESTS or DEEPSEEK_API_KEY"
            );
            return;
        }
        let provider = DeepSeekProvider::from_env().unwrap();
        let request = CompletionRequest {
            model: "deepseek-chat".to_string(),
            messages: vec![Message {
                role: Role::User,
                content: vec![Content::Text("你好".to_string())],
                name: None,
            }],
            options: CompletionOptions {
                temperature: Some(0.0),
                max_tokens: None,
                top_p: None,
                stream: true,
            },
            extensions: HashMap::new(),
        };
        let response = provider.complete_stream(request).await.unwrap();
        let mut stream = response.stream;
        let mut text = String::new();
        while let Some(item) = stream.next().await {
            let chunk = item.unwrap();
            for content in chunk.content {
                match content {
                    Content::Text(value) => text.push_str(&value),
                    Content::ImageUrl { .. } => {}
                }
            }
        }
        println!("{}", text);
        assert!(!text.is_empty());
    }
}
