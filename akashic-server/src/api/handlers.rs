use std::{convert::Infallible, time::Duration};

use agent::{
    agent::{CallModelEvent, call_model},
    core::Message,
};
use akashic_ecs::resources::task_manager::{TaskKind, TaskStatus, TaskUpdate};
use akashic_ecs::utils::build_chat_model;
use axum::{
    Json,
    extract::{Path, Query, State},
    http::HeaderMap,
    response::sse::{Event, KeepAlive, Sse},
    response::{IntoResponse, Response},
};
use futures::{StreamExt, stream};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

use crate::{error::AppError, state::AppState};

use super::dto::{
    ApiResponse, ControlGameSessionData, ControlGameSessionRequest, CreateGameSessionData,
    CreateGameSessionRequest, GameSessionWorldStateData, GenerateProfilesData,
    GenerateProfilesRequest, SessionPath,
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

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamQuery {
    since: Option<u64>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TaskUpdateData {
    event_id: Option<u64>,
    entity: String,
    kind: TaskKind,
    status: TaskStatus,
    chunk: Option<String>,
    output: Option<String>,
    error: Option<String>,
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

pub async fn create_game_session(
    State(state): State<AppState>,
    Json(request): Json<CreateGameSessionRequest>,
) -> ApiResult<CreateGameSessionData> {
    let session = state.create_game_session(request).await?;
    Ok(Json(ApiResponse::ok(session)))
}

pub async fn generate_profiles(
    Json(request): Json<GenerateProfilesRequest>,
) -> ApiResult<GenerateProfilesData> {
    if request.prompt.trim().is_empty() {
        return Err(AppError::bad_request("`prompt` 不能为空。"));
    }

    let mut model = build_chat_model();
    model.set_output_json(false);

    let messages = vec![
        Message::system(
            "你是一名世界观与角色设定助手。请根据用户提供的创意描述，生成两段纯文本内容。\
\n输出格式必须严格如下：\
\n[世界设定]\
\n这里写世界设定正文\
\n[主角设定]\
\n这里写主角设定正文\
\n不要输出 JSON，不要输出代码块，不要添加额外标题或解释。",
        ),
        Message::user(request.prompt),
    ];

    let mut stream = std::pin::pin!(call_model(&model, &messages, None));

    while let Some(event) = stream.next().await {
        match event {
            CallModelEvent::Completed { content, .. } => {
                let data = parse_generated_profiles(&content)
                    .ok_or_else(|| AppError::internal("模型返回格式不符合预期。"))?;
                return Ok(Json(ApiResponse::ok(data)));
            }
            CallModelEvent::Error(message) => {
                return Err(AppError::internal(format!("生成设定失败：{message}")));
            }
            CallModelEvent::TextChunk(_) | CallModelEvent::ReasoningChunk(_) => {}
        }
    }

    Err(AppError::internal("模型未返回完整结果。"))
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
    Query(query): Query<StreamQuery>,
    headers: HeaderMap,
) -> StorySseResult {
    let live_stream = state
        .open_game_session_stream(&path.session_id, query.since)
        .await?;
    let session_id = live_stream.session_id.clone();
    let _ = headers;

    let handshake_stream = stream::iter([Ok::<_, Infallible>(sse_json_event(
        "stream.handshake",
        StreamHandshakeData {
            session_id: session_id.clone(),
            protocol: "sse",
            note: "subscribed",
        },
    ))]);
    let live_stream = stream::unfold(
        Some((live_stream.event_rx, session_id)),
        |state| async move {
            let Some((mut event_rx, session_id)) = state else {
                return None;
            };

            match event_rx.recv().await {
                Ok(event) => Some((
                    Ok(task_updated_sse(None, event)),
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

    Ok(Sse::new(handshake_stream.chain(live_stream))
        .keep_alive(
            KeepAlive::new()
                .interval(Duration::from_secs(15))
                .text("keep-alive"),
        )
        .into_response())
}

fn task_updated_sse(event_id: Option<u64>, update: TaskUpdate) -> Event {
    let update = task_update_from_delta(event_id, update);
    let event = sse_json_event("task.updated", update);
    match event_id {
        Some(value) => event.id(value.to_string()),
        None => event,
    }
}

fn task_update_from_delta(event_id: Option<u64>, update: TaskUpdate) -> TaskUpdateData {
    TaskUpdateData {
        event_id,
        entity: update.entity,
        kind: update.kind,
        status: update.status,
        chunk: update.chunk,
        output: update.output,
        error: update.error,
    }
}

fn parse_generated_profiles(content: &str) -> Option<GenerateProfilesData> {
    let world_marker = "[世界设定]";
    let protagonist_marker = "[主角设定]";
    let world_start = content.find(world_marker)?;
    let protagonist_start = content.find(protagonist_marker)?;
    if protagonist_start <= world_start {
        return None;
    }

    let world = content[world_start + world_marker.len()..protagonist_start].trim();
    let protagonist = content[protagonist_start + protagonist_marker.len()..].trim();
    if world.is_empty() || protagonist.is_empty() {
        return None;
    }

    Some(GenerateProfilesData {
        world: world.to_string(),
        protagonist: protagonist.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::parse_generated_profiles;

    #[test]
    fn parse_generated_profiles_extracts_two_sections() {
        let result = parse_generated_profiles(
            "[世界设定]\n蒸汽与神谕并存的海上帝国。\n\n[主角设定]\n一名背负禁忌地图的年轻领航员。",
        )
        .expect("should parse");

        assert_eq!(result.world, "蒸汽与神谕并存的海上帝国。");
        assert_eq!(result.protagonist, "一名背负禁忌地图的年轻领航员。");
    }

    #[test]
    fn parse_generated_profiles_rejects_missing_sections() {
        assert!(parse_generated_profiles("只有一段内容").is_none());
    }
}
