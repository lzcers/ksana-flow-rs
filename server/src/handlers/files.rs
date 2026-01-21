use axum::{
    Json,
    extract::{Multipart, State, Path},
};
use serde_json::{Value, json};
use uuid::Uuid;

use crate::state::AppState;

pub async fn upload_file(
    Path(workspace_id): Path<String>,
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<Json<Value>, String> {
    while let Some(field) = multipart.next_field().await.map_err(|e| e.to_string())? {
        let name = field.name().unwrap_or("").to_string();
        let file_name = field.file_name().unwrap_or("unknown.txt").to_string();
        let content_type = field.content_type().unwrap_or("text/plain").to_string();

        if name == "file" {
            let data = field.text().await.map_err(|e| e.to_string())?;
            let size = data.len() as i64;
            let id = Uuid::new_v4().to_string();

            let db = state.db.lock().map_err(|_| "Failed to lock db")?;
            db.save_file(&id, &file_name, &data, size, &content_type, &workspace_id)
                .map_err(|e| e.to_string())?;

            return Ok(Json(json!({
                "id": id,
                "filename": file_name,
                "size": size,
                "type": content_type
            })));
        }
    }

    Err("No file uploaded".to_string())
}
