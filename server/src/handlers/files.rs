use axum::{
    Json,
    extract::{Multipart, State},
};
use serde_json::{Value, json};
use uuid::Uuid;

use crate::state::AppState;

pub async fn upload_file(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<Json<Value>, String> {
    let mut space_id = None;
    let mut file_data = None;

    while let Some(field) = multipart.next_field().await.map_err(|e| e.to_string())? {
        let name = field.name().unwrap_or("").to_string();

        if name == "space_id" {
            let val = field.text().await.map_err(|e| e.to_string())?;
            space_id = Some(val);
        } else if name == "file" {
            let file_name = field.file_name().unwrap_or("unknown.txt").to_string();
            let content_type = field.content_type().unwrap_or("text/plain").to_string();
            // Note: currently using text(), might need bytes() for binary
            let data = field.text().await.map_err(|e| e.to_string())?; 
            let size = data.len() as i64;
            file_data = Some((file_name, content_type, data, size));
        }
    }

    if let (Some(space_id), Some((file_name, content_type, data, size))) = (space_id, file_data) {
        let id = Uuid::new_v4().to_string();
        let db = state.db.lock().map_err(|_| "Failed to lock db")?;
        db.save_file(&id, &file_name, &data, size, &content_type, &space_id)
            .map_err(|e| e.to_string())?;

        return Ok(Json(json!({
            "id": id,
            "filename": file_name,
            "size": size,
            "type": content_type
        })));
    }

    Err("Missing space_id or file".to_string())
}
