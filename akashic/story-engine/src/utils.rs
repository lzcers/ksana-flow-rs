use std::{env, sync::Arc, time::Duration};

use agent::{models::ChatModel, providers::deepseek_provider};
use jsonrepair_rs::jsonrepair;
use serde::de::DeserializeOwned;

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
        .expect("设置 Akashic 模型失败");
    model.set_output_json(true);
    model.set_thinking_enabled(false);
    model
}

pub fn parse_json_response<T>(raw: &str) -> Result<T, String>
where
    T: DeserializeOwned,
{
    let cleaned = raw.trim();

    if let Ok(parsed) = serde_json::from_str::<T>(cleaned) {
        return Ok(parsed);
    }

    let repaired = jsonrepair(cleaned).map_err(|err| format!("无法解析 JSON 响应: {}", err))?;
    serde_json::from_str::<T>(&repaired).map_err(|_| format!("无法解析 JSON 响应: {}", cleaned))
}
