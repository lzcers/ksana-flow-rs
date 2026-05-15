use jsonrepair_rs::jsonrepair;
use std::{env, sync::Arc};

use agent::{
    agent::{AgentError, Layer, LayerKind, LayerMeta, StepResult},
    models::ChatModel,
    providers::deepseek_provider,
};
use bevy_ecs::{entity::Entity, message::MessageWriter};
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::time::Duration;

use crate::{
    resources::{task_manager::TaskResult, turn_state::TurnState},
    turn_messages::TurnEvent,
};

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

    let model_name = env::var("AKASHIC_MODEL").unwrap_or_else(|_| "deepseek-v4-flash".to_string());
    let http_timeout_secs = env::var("AKASHIC_HTTP_TIMEOUT_SECS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(660);
    let mut model = ChatModel::new();

    if let Ok(api_key) = env::var("DEEPSEEK_API_KEY") {
        let provider =
            deepseek_provider(api_key).with_timeout(Duration::from_secs(http_timeout_secs));
        model.add_models_for_provider(
            &["deepseek-v4-flash", "deepseek-v4-pro"],
            Arc::new(provider),
        );
    }
    model
        .set_active_model(&model_name)
        .expect("设置 Ak模型失败");
    model.set_output_json(true);
    model.set_thinking_enabled(false);
    // model.set_reasoning_effort("high");
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
    } else {
        let repaired = jsonrepair(cleaned).map_err(|err| format!("无法解析 JSON 响应: {}", err))?;
        if let Ok(parsed) = serde_json::from_str::<T>(&repaired) {
            return Ok(parsed);
        } else {
            return Err(format!("无法解析 JSON 响应: {}", cleaned));
        }
    }
}

pub fn task_success_output(task_result: &TaskResult) -> String {
    task_result
        .result
        .as_ref()
        .and_then(|result| result.as_ref().ok())
        .cloned()
        .unwrap_or_else(|| task_result.chunks.join(""))
}

pub fn task_error_message(task_result: &TaskResult, fallback: &str) -> String {
    task_result
        .result
        .as_ref()
        .and_then(|result| result.as_ref().err())
        .cloned()
        .unwrap_or_else(|| fallback.to_string())
}

pub fn write_task_failed(
    event_writer: &mut MessageWriter<TurnEvent>,
    turn_state: &TurnState,
    entity: Entity,
    message: String,
) {
    event_writer.write(TurnEvent::TaskFailed {
        turn_id: turn_state.active_turn_id,
        stage: turn_state.phase,
        entity,
        message,
    });
}

fn format_agent_error(error: &AgentError) -> String {
    error.to_string()
}
