use base64::Engine;
use flow::Input;
use rusqlite::{Connection, params};

use crate::config::get_config;

pub fn extract_string_input(input: &Input) -> String {
    input
        .get_str_as::<String>("input")
        .or_else(|| input.get_str_as::<String>("external_start"))
        .or_else(|| input.get_str_as::<String>("output"))
        .or_else(|| input.get_any_as::<String>())
        .unwrap_or_default()
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
