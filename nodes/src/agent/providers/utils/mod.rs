use super::super::core::{Content, Role};
use serde_json::{Value, json};

pub fn role_to_string(role: &Role) -> String {
    match role {
        Role::System => "system".to_string(),
        Role::User => "user".to_string(),
        Role::Assistant => "assistant".to_string(),
        Role::Tool => "tool".to_string(),
    }
}

pub fn content_to_text(contents: &[Content]) -> String {
    let mut text = String::new();
    for item in contents {
        match item {
            Content::Text(value) => text.push_str(value),
            Content::ImageUrl { .. } => {}
        }
    }
    text
}

pub fn content_to_value(contents: &[Content]) -> Value {
    if contents.is_empty() {
        return Value::String(String::new());
    }
    let all_text = contents.iter().all(|item| matches!(item, Content::Text(_)));
    if all_text {
        return Value::String(content_to_text(contents));
    }

    let mut parts = Vec::new();
    for item in contents {
        match item {
            Content::Text(value) => {
                parts.push(json!({ "type": "text", "text": value }));
            }
            Content::ImageUrl { url, detail } => {
                let mut image_url = serde_json::Map::new();
                image_url.insert("url".to_string(), json!(url));
                if let Some(detail) = detail {
                    image_url.insert("detail".to_string(), json!(detail));
                }
                parts.push(json!({ "type": "image_url", "image_url": image_url }));
            }
        }
    }
    Value::Array(parts)
}
