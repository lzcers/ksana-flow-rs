use std::{env, sync::Arc};

use agent::{
    agent::{AgentError, Layer, LayerKind, LayerMeta, StepResult},
    models::ChatModel,
    providers::{deepseek_provider_from_env, openrouter_provider_from_env},
};
use serde::de::DeserializeOwned;
use serde_json::Value;

pub fn build_layer(name: impl Into<String>, kind: LayerKind, data: Value, priority: i32) -> Layer {
    Layer {
        name: name.into(),
        kind,
        data,
        meta: LayerMeta {
            priority,
            ..LayerMeta::default()
        },
    }
}

pub fn build_chat_model() -> ChatModel {
    dotenv::dotenv().ok();

    let model_name = env::var("AKASHIC_MODEL").unwrap_or_else(|_| "deepseek-chat".to_string());
    let mut model = ChatModel::new();
    // model.set_output_json(true);

    if let Ok(provider) = deepseek_provider_from_env() {
        model.add_models_for_provider(&["deepseek-chat", "deepseek-reasoner"], Arc::new(provider));
    }
    model
        .set_active_model(&model_name).expect("设置 Ak模型失败");
    model
}

pub fn extract_step_content(result: StepResult) -> Result<String, String> {
    match result {
        StepResult::Done { content, .. } | StepResult::Continue { content, .. } => Ok(content),
        StepResult::Error(err) => Err(format_agent_error(&err)),
    }
}

pub fn parse_json_response<T>(raw: &str) -> Result<T, String>
where
    T: DeserializeOwned,
{
    let cleaned = raw.trim();

    if let Ok(parsed) = serde_json::from_str::<T>(cleaned) {
        return Ok(parsed);
    }

    if let Some(start) = cleaned.find('{')
        && let Some(end) = cleaned.rfind('}')
    {
        let slice = &cleaned[start..=end];
        if let Ok(parsed) = serde_json::from_str::<T>(slice) {
            return Ok(parsed);
        }
    }

    Err(format!("无法解析 JSON 响应: {}", cleaned))
}

fn format_agent_error(error: &AgentError) -> String {
    error.to_string()
}
