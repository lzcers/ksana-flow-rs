use std::{convert::Infallible, time::Duration};

use akashic_ecs::resources::export::TaskEvent;
use akashic_ecs::resources::{
    export::TaskView,
    task_manager::{TaskKind, TaskStatus},
};
use axum::{
    Json,
    extract::{Path, State},
    response::sse::{Event, KeepAlive, Sse},
    response::{IntoResponse, Response},
};
use futures::{StreamExt, stream};
use serde::Serialize;
use tokio::sync::broadcast;

use crate::{error::AppError, state::AppState};

use super::dto::{
    ApiResponse, ControlGameSessionData, ControlGameSessionRequest, CreateGameSessionData,
    CreateGameSessionRequest, GameSessionWorldStateData, HealthzData, SessionPath,
};

type ApiResult<T> = Result<Json<ApiResponse<T>>, AppError>;
type StorySseResult = Result<Response, AppError>;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct StreamDoneData {
    route: &'static str,
    session_id: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct StreamWarningData {
    session_id: String,
    reason: &'static str,
    skipped: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct StreamHandshakeData {
    session_id: String,
    protocol: &'static str,
    note: &'static str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TaskUpdateData {
    entity: String,
    kind: TaskKind,
    status: TaskStatus,
    chunk: Option<String>,
    output: Option<String>,
    error: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TaskUpdatedData {
    task: TaskView,
    update: TaskUpdateData,
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

pub async fn healthz(State(state): State<AppState>) -> Json<ApiResponse<HealthzData>> {
    let _ = state;
    Json(ApiResponse::ok(HealthzData {
        status: "ok".to_string(),
        service_name: "akashic-server".to_string(),
        api_version: "mvp".to_string(),
    }))
}

pub async fn create_game_session(
    State(state): State<AppState>,
    Json(request): Json<CreateGameSessionRequest>,
) -> ApiResult<CreateGameSessionData> {
    let session = state.create_game_session(request).await?;
    Ok(Json(ApiResponse::ok(session)))
}

pub async fn get_game_session_world(
    State(state): State<AppState>,
    Path(path): Path<SessionPath>,
) -> ApiResult<GameSessionWorldStateData> {
    let state_view = state.get_game_session_world(&path.session_id).await?;
    Ok(Json(ApiResponse::ok(state_view)))
}

pub async fn control_game_session(
    State(state): State<AppState>,
    Path(path): Path<SessionPath>,
    Json(request): Json<ControlGameSessionRequest>,
) -> ApiResult<ControlGameSessionData> {
    let result = state
        .control_game_session(&path.session_id, request)
        .await?;
    Ok(Json(ApiResponse::ok(result)))
}

pub async fn stream_game_session(
    State(state): State<AppState>,
    Path(path): Path<SessionPath>,
) -> StorySseResult {
    let live_stream = state.open_game_session_stream(&path.session_id).await?;
    let session_id = live_stream.session_id.clone();

    let handshake_stream = stream::iter([Ok::<_, Infallible>(sse_json_event(
        "stream.handshake",
        StreamHandshakeData {
            session_id: session_id.clone(),
            protocol: "sse",
            note: "subscribed",
        },
    ))]);
    let initial_events = live_stream
        .tasks
        .into_iter()
        .map(|task| Ok::<_, Infallible>(task_updated_sse(task)));

    let initial_stream = stream::iter(initial_events);
    let live_stream = stream::unfold(
        Some((live_stream.event_rx, session_id)),
        |state| async move {
            let Some((mut event_rx, session_id)) = state else {
                return None;
            };

            match event_rx.recv().await {
                Ok(event) => Some((
                    Ok(session_event_to_sse(event)),
                    Some((event_rx, session_id)),
                )),
                Err(broadcast::error::RecvError::Lagged(skipped)) => Some((
                    Ok(sse_json_event(
                        "stream.warning",
                        StreamWarningData {
                            session_id: session_id.clone(),
                            reason: "lagged",
                            skipped,
                        },
                    )),
                    Some((event_rx, session_id)),
                )),
                Err(broadcast::error::RecvError::Closed) => Some((
                    Ok(sse_done_event("stream_game_session.done", Some(session_id))),
                    None,
                )),
            }
        },
    );

    Ok(
        Sse::new(handshake_stream.chain(initial_stream).chain(live_stream))
            .keep_alive(
                KeepAlive::new()
                    .interval(Duration::from_secs(15))
                    .text("keep-alive"),
            )
            .into_response(),
    )
}

fn session_event_to_sse(event: TaskEvent) -> Event {
    match event {
        TaskEvent::TaskUpdated { task } => task_updated_sse(task),
    }
}

fn task_updated_sse(task: TaskView) -> Event {
    let update = task_update_from_view(&task);
    sse_json_event("task.updated", TaskUpdatedData { task, update })
}

fn task_update_from_view(task: &TaskView) -> TaskUpdateData {
    TaskUpdateData {
        entity: task.entity.clone(),
        kind: task.kind,
        status: task.status,
        chunk: match task.status {
            TaskStatus::Running => task.chunks.last().cloned(),
            _ => None,
        },
        output: match task.status {
            TaskStatus::Done => task.output.clone(),
            _ => None,
        },
        error: match task.status {
            TaskStatus::Error => task.error.clone().or_else(|| task.last_error.clone()),
            _ => None,
        },
    }
}
