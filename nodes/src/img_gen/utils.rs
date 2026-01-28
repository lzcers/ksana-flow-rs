use base64::Engine;
use flow::NodeInputs;
use rusqlite::{Connection, params};
use serde_json::{Value, json};

use crate::config::get_config;

pub fn extract_string_input(inputs: &NodeInputs) -> String {
    inputs
        .get::<String>("input")
        .or_else(|| inputs.get::<String>("external_start"))
        .or_else(|| inputs.get::<String>("output"))
        .or_else(|| {
            inputs
                .get_any()
                .and_then(|p| p.as_any())
                .and_then(|a| a.downcast_ref::<String>())
        })
        .cloned()
        .unwrap_or_default()
}

pub fn build_payload(
    model: &str,
    system_prompt: &str,
    prompt: &str,
    image_data_url: Option<String>,
    aspect_ratio: &str,
    image_size: &str,
) -> Value {
    let mut messages: Vec<Value> = Vec::new();
    if !system_prompt.is_empty() {
        messages.push(json!({
            "role": "system",
            "content": system_prompt
        }));
    }

    if let Some(data_url) = image_data_url {
        messages.push(json!({
            "role": "user",
            "content": [
                { "type": "image_url", "image_url": { "url": data_url } }
                ,{ "type": "text", "text": prompt }
            ]
        }));
    } else {
        messages.push(json!({
            "role": "user",
            "content": prompt
        }));
    }

    json!({
        "model": model,
        "messages": messages,
        "modalities": ["image", "text"],
        "stream": false,
        "image_config": {
            "aspect_ratio": aspect_ratio,
            "image_size": image_size
        }
    })
}

pub async fn send_openrouter_chat_completions(
    http: &reqwest::Client,
    base_url: &str,
    api_key: &str,
    payload: &Value,
) -> Result<Value, String> {
    let url = format!("{}/chat/completions", base_url.trim_end_matches('/'));
    let mut req = http.post(url).bearer_auth(api_key).json(payload);

    if let Ok(referer) = std::env::var("OPENROUTER_HTTP_REFERER") {
        req = req.header("HTTP-Referer", referer);
    }
    if let Ok(title) = std::env::var("OPENROUTER_X_TITLE") {
        req = req.header("X-Title", title);
    }

    let resp = req
        .send()
        .await
        .map_err(|e| format!("OpenRouter request failed: {}", e))?;
    let status = resp.status();
    let resp_json: Value = resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse OpenRouter response: {}", e))?;

    if !status.is_success() {
        return Err(format!("OpenRouter error {}: {}", status, resp_json));
    }

    Ok(resp_json)
}

pub fn extract_first_image_data_url(response: &Value) -> Option<String> {
    let message = response.get("choices")?.get(0)?.get("message")?;

    if let Some(url) = message
        .get("images")
        .and_then(|v| v.get(0))
        .and_then(|v| v.get("image_url"))
        .and_then(|v| v.get("url"))
        .and_then(|v| v.as_str())
    {
        return Some(url.to_string());
    }

    match message.get("content") {
        Some(Value::String(s)) => {
            if s.trim_start().starts_with("data:image/") {
                return Some(s.to_string());
            }
        }
        Some(Value::Array(parts)) => {
            for part in parts {
                let Some(part_type) = part.get("type").and_then(|v| v.as_str()) else {
                    continue;
                };
                if part_type == "image_url" {
                    if let Some(url) = part
                        .get("image_url")
                        .and_then(|v| v.get("url"))
                        .and_then(|v| v.as_str())
                    {
                        return Some(url.to_string());
                    }
                }
            }
        }
        _ => {}
    }

    None
}

pub fn extract_model_error(response: &Value) -> Option<String> {
    response.get("error").map(|v| v.to_string())
}

pub fn response_debug_excerpt(response: &Value) -> String {
    let excerpt = response
        .get("choices")
        .and_then(|v| v.get(0))
        .and_then(|v| v.get("message"))
        .cloned()
        .unwrap_or_else(|| response.clone());

    let s = serde_json::to_string(&excerpt).unwrap_or_else(|_| excerpt.to_string());
    const MAX: usize = 2000;
    if s.len() <= MAX {
        s
    } else {
        format!("{}...(truncated)", &s[..MAX])
    }
}

pub fn parse_data_url_to_bytes(data_url: &str) -> Result<(String, Vec<u8>), String> {
    let (meta, b64) = data_url
        .split_once(',')
        .ok_or_else(|| "Invalid data URL".to_string())?;
    let meta = meta
        .strip_prefix("data:")
        .ok_or_else(|| "Invalid data URL".to_string())?;

    let mime_type = meta.strip_suffix(";base64").unwrap_or(meta).to_string();

    let bytes = base64::engine::general_purpose::STANDARD
        .decode(b64)
        .map_err(|e| format!("Base64 decode failed: {}", e))?;

    Ok((mime_type, bytes))
}

pub fn load_uploaded_image_data_url(
    file_id: &str,
    space_id: &str,
) -> Result<Option<String>, String> {
    let config = get_config().map_err(|e| format!("Error loading config: {}", e))?;
    let conn = Connection::open(&config.source.data_db_uri)
        .map_err(|e| format!("Error opening DB: {}", e))?;

    let mut stmt = conn
        .prepare(
            "SELECT mime_type, content, content_blob FROM uploaded_files WHERE id = ?1 AND workspace_id = ?2",
        )
        .map_err(|e| format!("Error preparing statement: {}", e))?;

    let row: Result<(String, String, Option<Vec<u8>>), _> = stmt
        .query_row(params![file_id, space_id], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?))
        });

    let (mime_type, content_text, content_blob) = match row {
        Ok(v) => v,
        Err(rusqlite::Error::QueryReturnedNoRows) => return Ok(None),
        Err(e) => return Err(format!("Error reading uploaded file: {}", e)),
    };

    if let Some(bytes) = content_blob {
        let b64 = base64::engine::general_purpose::STANDARD.encode(bytes);
        return Ok(Some(format!("data:{};base64,{}", mime_type, b64)));
    }

    if content_text.starts_with("data:") {
        return Ok(Some(content_text));
    }

    if mime_type.starts_with("image/") && !content_text.is_empty() {
        return Ok(Some(format!("data:{};base64,{}", mime_type, content_text)));
    }

    Ok(None)
}

pub fn save_ai_media_to_db(
    id: &str,
    mime_type: &str,
    data: &[u8],
    prompt: &str,
    model: &str,
    space_id: &str,
) -> Result<(), String> {
    let config = get_config().map_err(|e| format!("Error loading config: {}", e))?;
    let conn = Connection::open(&config.source.data_db_uri)
        .map_err(|e| format!("Error opening DB: {}", e))?;

    conn.execute(
        "INSERT INTO ai_media (id, kind, mime_type, size, data, prompt, model, workspace_id) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![id, "image", mime_type, data.len() as i64, data, prompt, model, space_id],
    )
    .map_err(|e| format!("Error saving ai_media: {}", e))?;

    Ok(())
}
