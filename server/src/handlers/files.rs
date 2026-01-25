use axum::{
    Json,
    extract::{Multipart, Path, Query, State},
    http::{StatusCode, header},
    response::Response,
};
use serde::Deserialize;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::state::AppState;

#[derive(Deserialize)]
pub struct SpaceQuery {
    pub space_id: String,
}

pub async fn upload_file(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<Json<Value>, String> {
    let mut space_id = None;
    let mut file_data: Option<(String, String, FileUploadData)> = None;

    while let Some(field) = multipart.next_field().await.map_err(|e| e.to_string())? {
        let name = field.name().unwrap_or("").to_string();

        if name == "space_id" {
            let val = field.text().await.map_err(|e| e.to_string())?;
            space_id = Some(val);
        } else if name == "file" {
            let file_name = field.file_name().unwrap_or("unknown.txt").to_string();
            let content_type = field
                .content_type()
                .unwrap_or("application/octet-stream")
                .to_string();

            let upload_data = if is_text_mime(&content_type) {
                let data = field.text().await.map_err(|e| e.to_string())?;
                FileUploadData::Text(data)
            } else {
                let data = field.bytes().await.map_err(|e| e.to_string())?.to_vec();
                FileUploadData::Bytes(data)
            };

            file_data = Some((file_name, content_type, upload_data));
        }
    }

    if let (Some(space_id), Some((file_name, content_type, upload_data))) = (space_id, file_data) {
        let id = Uuid::new_v4().to_string();
        let db = state.db.lock().map_err(|_| "Failed to lock db")?;
        let size = match &upload_data {
            FileUploadData::Text(s) => s.len() as i64,
            FileUploadData::Bytes(b) => b.len() as i64,
        };

        match upload_data {
            FileUploadData::Text(data) => {
                db.save_file(&id, &file_name, &data, size, &content_type, &space_id)
                    .map_err(|e| e.to_string())?;
            }
            FileUploadData::Bytes(data) => {
                db.save_file_bytes(&id, &file_name, &data, size, &content_type, &space_id)
                    .map_err(|e| e.to_string())?;
            }
        }

        return Ok(Json(json!({
            "id": id,
            "filename": file_name,
            "size": size,
            "type": content_type
        })));
    }

    Err("Missing space_id or file".to_string())
}

enum FileUploadData {
    Text(String),
    Bytes(Vec<u8>),
}

fn is_text_mime(mime: &str) -> bool {
    mime.starts_with("text/")
        || mime == "application/json"
        || mime == "application/xml"
        || mime.ends_with("+json")
        || mime.ends_with("+xml")
}

pub async fn get_file(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(q): Query<SpaceQuery>,
) -> Result<Response, (StatusCode, String)> {
    let db = state.db.lock().map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to lock db".to_string(),
        )
    })?;

    let Some((filename, mime_type, content_text, content_blob)) = db
        .get_file_data(&id, &q.space_id)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    else {
        return Err((StatusCode::NOT_FOUND, "File not found".to_string()));
    };

    let bytes = content_blob.unwrap_or_else(|| content_text.into_bytes());

    let response = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, mime_type)
        .header(
            header::CONTENT_DISPOSITION,
            format!("inline; filename=\"{}\"", filename),
        )
        .body(axum::body::Body::from(bytes))
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(response)
}

pub async fn get_ai_media(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(q): Query<SpaceQuery>,
) -> Result<Response, (StatusCode, String)> {
    let db = state.db.lock().map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to lock db".to_string(),
        )
    })?;

    let Some((mime_type, data)) = db
        .get_ai_media(&id, &q.space_id)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    else {
        return Err((StatusCode::NOT_FOUND, "Media not found".to_string()));
    };

    let response = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, mime_type)
        .body(axum::body::Body::from(data))
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(response)
}
