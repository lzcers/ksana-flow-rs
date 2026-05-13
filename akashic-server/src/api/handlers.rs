use std::{convert::Infallible, time::Duration};

use axum::{
    Json,
    extract::{Path, State},
    response::sse::{Event, KeepAlive, Sse},
    response::{IntoResponse, Response},
};
use futures::stream;
use serde::Serialize;

use crate::{error::AppError, state::AppState};

use super::dto::{
    ApiResponse, ArchiveDetailData, ArchiveListData, ArchivePath, CreateGameSessionRequest,
    CreateSaveRequest, GenerateEndingShareCardRequest, GenerateSaveShareCardRequest, HealthzData,
    IntuitionPreviewRequest, LoadSaveData, SaveListData, SavePath, SaveSummary, SessionPath,
    ShareCardData, SubmitChoiceRequest,
};

type ApiResult<T> = Result<Json<ApiResponse<T>>, AppError>;
type StorySseResult = Result<Response, AppError>;

#[derive(Serialize)]
struct StreamDoneData {
    route: &'static str,
    session_id: Option<String>,
}

#[derive(Serialize)]
struct HistoryMetaData {
    session_id: String,
    total_items: usize,
}

fn sse_json_event<T>(name: &str, data: T) -> Event
where
    T: Serialize,
{
    Event::default()
        .event(name)
        .json_data(data)
        .expect("failed to serialize SSE event")
}

fn sse_done_event(route: &'static str, session_id: Option<String>) -> Event {
    sse_json_event(route, StreamDoneData { route, session_id })
}

fn sse_response(events: Vec<Event>) -> Response {
    let stream = stream::iter(events.into_iter().map(Ok::<_, Infallible>));
    Sse::new(stream)
        .keep_alive(
            KeepAlive::new()
                .interval(Duration::from_secs(15))
                .text("keep-alive"),
        )
        .into_response()
}

pub async fn healthz(State(state): State<AppState>) -> Json<ApiResponse<HealthzData>> {
    Json(ApiResponse::ok(HealthzData {
        status: "ok".to_string(),
        service_name: state.service_name.to_string(),
        api_version: state.api_version.to_string(),
    }))
}

pub async fn create_game_session(
    State(state): State<AppState>,
    Json(request): Json<CreateGameSessionRequest>,
) -> StorySseResult {
    let session = state.create_game_session(request)?;
    let session_id = session.session_id.clone();

    Ok(sse_response(vec![
        sse_json_event("session.created", session),
        sse_done_event("create_game_session.done", Some(session_id)),
    ]))
}

pub async fn get_game_session(
    State(state): State<AppState>,
    Path(path): Path<SessionPath>,
) -> StorySseResult {
    let snapshot = state.get_game_session(&path.session_id)?;
    let session_id = snapshot.session_id.clone();

    Ok(sse_response(vec![
        sse_json_event("session.snapshot", snapshot),
        sse_done_event("get_game_session.done", Some(session_id)),
    ]))
}

pub async fn submit_choice(
    State(state): State<AppState>,
    Path(path): Path<SessionPath>,
    Json(request): Json<SubmitChoiceRequest>,
) -> StorySseResult {
    let session_id = path.session_id;
    let submission = state.submit_choice(&session_id, request)?;
    let snapshot = state.get_game_session(&session_id)?;
    let ending = if snapshot.ending_status == "ready" {
        Some(state.get_game_session_ending(&session_id)?)
    } else {
        None
    };

    let mut events = vec![
        sse_json_event("choice.submitted", submission),
        sse_json_event("session.snapshot", snapshot),
    ];
    if let Some(ending) = ending {
        events.push(sse_json_event("ending.ready", ending));
    }
    events.push(sse_done_event("submit_choice.done", Some(session_id)));

    Ok(sse_response(events))
}

pub async fn get_game_session_ending(
    State(state): State<AppState>,
    Path(path): Path<SessionPath>,
) -> StorySseResult {
    let ending = state.get_game_session_ending(&path.session_id)?;
    let session_id = ending.session_id.clone();

    Ok(sse_response(vec![
        sse_json_event("ending.ready", ending),
        sse_done_event("get_game_session_ending.done", Some(session_id)),
    ]))
}

pub async fn create_intuition_preview(
    State(state): State<AppState>,
    Path(path): Path<SessionPath>,
    Json(request): Json<IntuitionPreviewRequest>,
) -> StorySseResult {
    let preview = state.create_intuition_preview(&path.session_id, &request.choice_id)?;

    Ok(sse_response(vec![
        sse_json_event("intuition.preview", preview),
        sse_done_event("create_intuition_preview.done", Some(path.session_id)),
    ]))
}

pub async fn get_game_session_history(
    State(state): State<AppState>,
    Path(path): Path<SessionPath>,
) -> StorySseResult {
    let history = state.get_game_session_history(&path.session_id)?;
    let total_items = history.items.len();

    let mut events = Vec::with_capacity(total_items + 2);
    events.push(sse_json_event(
        "history.started",
        HistoryMetaData {
            session_id: path.session_id.clone(),
            total_items,
        },
    ));
    for item in history.items {
        events.push(sse_json_event("history.item", item));
    }
    events.push(sse_done_event(
        "get_game_session_history.done",
        Some(path.session_id),
    ));

    Ok(sse_response(events))
}

pub async fn stream_game_session(
    State(state): State<AppState>,
    Path(path): Path<SessionPath>,
) -> StorySseResult {
    let session_id = path.session_id;
    let handshake = state.stream_game_session(&session_id)?;
    let snapshot = state.get_game_session(&session_id)?;
    let history = state.get_game_session_history(&session_id)?;
    let ending = if snapshot.ending_status == "ready" {
        Some(state.get_game_session_ending(&session_id)?)
    } else {
        None
    };

    let mut events = Vec::with_capacity(history.items.len() + 4);
    events.push(sse_json_event("stream.handshake", handshake));
    events.push(sse_json_event("session.snapshot", snapshot));
    for item in history.items {
        events.push(sse_json_event("history.item", item));
    }
    if let Some(ending) = ending {
        events.push(sse_json_event("ending.ready", ending));
    }
    events.push(sse_done_event("stream_game_session.done", Some(session_id)));

    Ok(sse_response(events))
}

pub async fn create_save(
    State(state): State<AppState>,
    Json(request): Json<CreateSaveRequest>,
) -> ApiResult<SaveSummary> {
    Ok(Json(ApiResponse::ok(state.create_save(request)?)))
}

pub async fn list_saves(State(state): State<AppState>) -> ApiResult<SaveListData> {
    Ok(Json(ApiResponse::ok(state.list_saves())))
}

pub async fn load_save(
    State(state): State<AppState>,
    Path(path): Path<SavePath>,
) -> ApiResult<LoadSaveData> {
    Ok(Json(ApiResponse::ok(state.load_save(&path.save_id)?)))
}

pub async fn list_archives(State(state): State<AppState>) -> ApiResult<ArchiveListData> {
    Ok(Json(ApiResponse::ok(state.list_archives())))
}

pub async fn get_archive(
    State(state): State<AppState>,
    Path(path): Path<ArchivePath>,
) -> ApiResult<ArchiveDetailData> {
    Ok(Json(ApiResponse::ok(state.get_archive(&path.archive_id)?)))
}

pub async fn generate_save_share_card(
    State(state): State<AppState>,
    Json(request): Json<GenerateSaveShareCardRequest>,
) -> ApiResult<ShareCardData> {
    Ok(Json(ApiResponse::ok(state.generate_save_share_card(
        &request.save_id,
        &request.style,
    )?)))
}

pub async fn generate_ending_share_card(
    State(state): State<AppState>,
    Json(request): Json<GenerateEndingShareCardRequest>,
) -> ApiResult<ShareCardData> {
    Ok(Json(ApiResponse::ok(state.generate_ending_share_card(
        &request.archive_id,
        &request.style,
    )?)))
}
