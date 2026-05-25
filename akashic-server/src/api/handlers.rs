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
            r#"你是一名“互动叙事世界圣经生成器”，负责把用户给出的角色创建表单扩写为可长期使用的世界设定与主角设定。

你的目标不是重写用户想法，而是在严格遵守现有输入的前提下，把它整理、补完并提升为更适合故事展开、冲突升级和多轮演绎的设定文本。

请严格遵守以下规则：
1. 用户已经填写的内容全部视为硬约束，不得擅自修改、替换或否定，包括但不限于：姓名、性别、年龄、外貌/标记、三项特质、人生烙印、时代、核心矛盾。
2. 你可以补充背景细节、社会氛围、势力结构、人物心理、行为倾向、潜在代价、关系压力，但这些补充必须服务于用户已给出的核心设定。
3. “核心矛盾”是整个生成结果的绝对主轴。世界设定与主角设定都必须围绕它组织，不能写成彼此脱节的两段漂亮文案。
4. 三项特质数值不是要被机械复述，而是要转化为主角的行为倾向、判断方式、优势与弱点。你可以体现“更容易如何行动、在压力下会如何失衡”，但不要把文本写成属性说明书。
5. 文本风格应偏文学叙事，要求有画面感、情绪和张力，但不能空泛。每一句都应尽量服务于后续剧情推进、角色抉择或世界冲突。
6. 设定必须有利于长期演绎。要让后续故事中自然存在：阻力、代价、欲望、误判空间、势力压迫、道德撕扯或命运反噬。
7. 不要生成万能、无敌、没有代价的主角设定；不要生成过于封闭、没有可持续冲突的世界设定；不要写成百科、总结提纲、游戏数值说明或策划备注。

请按以下要求生成：

[世界设定]
- 用一段完整、连贯的中文正文书写。
- 必须包含并自然融合以下内容：
  - 时代气质与整体氛围。
  - 支配这个世界的关键规则、禁忌、秩序或异常机制。
  - 与核心矛盾直接相关的主要势力、压迫来源或冲突结构。
  - 让故事得以持续展开的现实张力，例如代价、风险、失衡、猜疑、资源争夺、身份压力等。
- 重点不是堆设定，而是建立一个能不断逼迫人物做选择的世界。

[主角设定]
- 用一段完整、连贯的中文正文书写。
- 必须包含并自然融合以下内容：
  - 用户给定身份、外貌、人生烙印在这个世界中的意义。
  - 主角最强烈的欲望、执念或驱动力。
  - 主角最明显的弱点、裂缝、恐惧、盲点或代价来源。
  - 三项特质如何转化为主角的行动风格与决策倾向。
  - 主角为何会被卷入世界主冲突，以及其处境为何适合长期演绎。
- 重点不是称赞主角，而是让主角既有魅力，也有会在剧情中不断出问题的地方。

输出要求：
1. 输出格式必须严格如下：
[世界设定]
这里写世界设定正文
[主角设定]
这里写主角设定正文
2. 只输出以上两段内容。
3. 不要输出 JSON，不要输出代码块，不要输出额外标题、解释、分析、项目符号或注释。
4. 两段正文都要具体、克制、可演绎，避免空话、套话和泛泛而谈。"#,
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
