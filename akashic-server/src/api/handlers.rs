use std::{convert::Infallible, time::Duration};

use akashic_ecs::resources::export::SessionEvent;
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
    ApiResponse, CreateGameSessionData, CreateGameSessionRequest, HealthzData, SessionPath,
    SubmitChoiceRequest,
};

type ApiResult<T> = Result<Json<ApiResponse<T>>, AppError>;
type StorySseResult = Result<Response, AppError>;

#[derive(Serialize)]
struct StreamDoneData {
    route: &'static str,
    session_id: Option<String>,
}

#[derive(Serialize)]
struct StreamWarningData {
    session_id: String,
    reason: &'static str,
    skipped: u64,
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

pub async fn get_game_session(
    State(state): State<AppState>,
    Path(path): Path<SessionPath>,
) -> StorySseResult {
    let snapshot = state.get_game_session(&path.session_id).await?;
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
    let submission = state.submit_choice(&session_id, request).await?;
    let snapshot = state.get_game_session(&session_id).await?;
    let ending = if snapshot.ending_status == "ready" {
        Some(state.get_game_session_ending(&session_id).await?)
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
    let ending = state.get_game_session_ending(&path.session_id).await?;
    let session_id = ending.session_id.clone();

    Ok(sse_response(vec![
        sse_json_event("ending.ready", ending),
        sse_done_event("get_game_session_ending.done", Some(session_id)),
    ]))
}

pub async fn stream_game_session(
    State(state): State<AppState>,
    Path(path): Path<SessionPath>,
) -> StorySseResult {
    let live_stream = state.open_game_session_stream(&path.session_id).await?;
    let session_id = live_stream.handshake.session_id.clone();

    let mut initial_events = Vec::with_capacity(live_stream.history.items.len() + 3);
    initial_events.push(sse_json_event("stream.handshake", live_stream.handshake));
    initial_events.push(sse_json_event("session.snapshot", live_stream.snapshot));
    for item in live_stream.history.items {
        initial_events.push(sse_json_event("history.item", item));
    }
    if let Some(ending) = live_stream.ending {
        initial_events.push(sse_json_event("ending.ready", ending));
    }

    let initial_stream = stream::iter(initial_events.into_iter().map(Ok::<_, Infallible>));
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

    Ok(Sse::new(initial_stream.chain(live_stream))
        .keep_alive(
            KeepAlive::new()
                .interval(Duration::from_secs(15))
                .text("keep-alive"),
        )
        .into_response())
}

fn session_event_to_sse(event: SessionEvent) -> Event {
    match event {
        SessionEvent::TurnChanged {
            phase,
            turn_index,
            active_turn_id,
        } => sse_json_event("turn.changed", phase),
        SessionEvent::WorldSnapshotUpdated { world } => sse_json_event("world.updated", world),
        SessionEvent::TaskUpdated { task, update } => sse_json_event(
            "task.updated",
            serde_json::json!({
                "task": task,
                "update": update,
            }),
        ),
    }
}
