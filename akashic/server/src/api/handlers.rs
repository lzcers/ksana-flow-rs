use std::{convert::Infallible, time::Duration};

use agent::{
    agent::{CallModelEvent, call_model},
    core::Message,
};
use akashic_ecs::resources::agent_task::{TaskChunkKind, TaskKind, TaskStatus, TaskUpdate};
use akashic_ecs::utils::{build_chat_model, parse_json_response};
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
    CreateGameSessionRequest, CreateSaveSlotData, CreateSaveSlotRequest, GameSessionWorldStateData,
    GenerateProfilesData, GenerateProfilesRequest, LoadArchiveRequest, LoadGameSessionRequest,
    SaveExportData, SessionPath, StorySummaryData,
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
    chunk_kind: Option<TaskChunkKind>,
    chunk: Option<String>,
    output: Option<String>,
    error: Option<String>,
}

#[derive(Deserialize)]
struct StorySummaryPayload {
    summary: String,
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

pub async fn create_save_slot(
    State(state): State<AppState>,
    Path(path): Path<SessionPath>,
    Json(request): Json<CreateSaveSlotRequest>,
) -> ApiResult<CreateSaveSlotData> {
    let saved = state
        .create_save_slot(&path.session_id, request.title.as_deref())
        .await?;
    Ok(Json(ApiResponse::ok(saved)))
}

pub async fn save_export(
    State(state): State<AppState>,
    Path(path): Path<SessionPath>,
    Json(request): Json<CreateSaveSlotRequest>,
) -> ApiResult<SaveExportData> {
    let exported = state
        .export_save_archive(&path.session_id, request.title.as_deref())
        .await?;
    Ok(Json(ApiResponse::ok(exported)))
}

pub async fn load_game_session(
    State(state): State<AppState>,
    Json(request): Json<LoadGameSessionRequest>,
) -> ApiResult<GameSessionWorldStateData> {
    let slot_id = request.slot_id.trim();
    if slot_id.is_empty() {
        return Err(AppError::bad_request("`slotId` 不能为空。"));
    }

    let session = state.load_game_session_from_slot(slot_id).await?;
    Ok(Json(ApiResponse::ok(session)))
}

pub async fn load_archive(
    State(state): State<AppState>,
    Json(request): Json<LoadArchiveRequest>,
) -> ApiResult<GameSessionWorldStateData> {
    let session = state
        .load_game_session_from_archive(request.compressed_archive)
        .await?;
    Ok(Json(ApiResponse::ok(session)))
}

pub async fn generate_profiles(
    Json(request): Json<GenerateProfilesRequest>,
) -> ApiResult<GenerateProfilesData> {
    if request.prompt.trim().is_empty() {
        return Err(AppError::bad_request("`prompt` 不能为空。"));
    }

    let mut model = build_chat_model();
    model.set_output_json(true);

    let messages = vec![
        Message::system(
            r#"你是一名“互动叙事世界圣经生成器”，负责把用户给出的角色创建表单扩写为可长期使用的世界设定与主角设定。

你的目标不是重写用户想法，而是在严格遵守现有输入的前提下，把它整理、补完并提升为更适合故事展开、冲突升级和多轮演绎的设定文本。

请严格遵守以下规则：
1. 用户已经填写的内容全部视为硬约束，不得擅自修改、替换或否定，包括但不限于：姓名、性别、年龄、外貌/标记、三项特质、人生烙印、时代。
2. 你可以补充背景细节、社会氛围、势力结构、人物心理、行为倾向、潜在代价、关系压力，但这些补充必须服务于用户已给出的核心设定。
3. 三项特质数值不是要被机械复述，而是要转化为主角的行为倾向、判断方式、优势与弱点。你可以体现“更容易如何行动、在压力下会如何失衡”，但不要把文本写成属性说明书。
4. 文本风格应偏文学叙事，要求有画面感、情绪和张力，但不能空泛。每一句都应尽量服务于后续剧情推进、角色抉择或世界冲突。
5. 设定必须有利于长期演绎。要让后续故事中自然存在：阻力、代价、欲望、误判空间、势力压迫、道德撕扯或命运反噬。
6. 不要生成万能、无敌、没有代价的主角设定；不要生成过于封闭、没有可持续冲突的世界设定；不要写成百科、总结提纲、游戏数值说明或策划备注。

请按以下 JSON 结构生成，字段名必须完全一致：
{
  "world": "世界设定正文",
  "protagonist": "主角设定正文",
  "keyStoryBeats": "关键节点骨架，多行字符串，每行一个节点"
}

字段要求：
1. `world`：
- 用一段完整、连贯的中文正文书写。
- 必须包含并自然融合以下内容：
  - 时代气质与整体氛围。
  - 支配这个世界的关键规则、禁忌、秩序或异常机制。
  - 与核心矛盾直接相关的主要势力、压迫来源或冲突结构。
  - 让故事得以持续展开的现实张力，例如代价、风险、失衡、猜疑、资源争夺、身份压力等。
- 重点不是堆设定，而是建立一个能不断逼迫人物做选择的世界。

2. `protagonist`：
- 用一段完整、连贯的中文正文书写。
- 必须包含并自然融合以下内容：
  - 用户给定身份、外貌、人生烙印在这个世界中的意义。
  - 主角最强烈的欲望、执念或驱动力。
  - 主角最明显的弱点、裂缝、恐惧、盲点或代价来源。
  - 三项特质如何转化为主角的行动风格与决策倾向。
  - 主角为何会被卷入世界主冲突，以及其处境为何适合长期演绎。
- 重点不是称赞主角，而是让主角既有魅力，也有会在剧情中不断出问题的地方。

3. `keyStoryBeats`：
- 写成 4 到 6 行的多行字符串，每行一个关键节点，不要编号。
- 每一条都应是故事未来必须抵达、对角色与世界都具有决定性影响的关键场景、真相揭示、关系断裂、代价兑现或终局瞬间。
- 这些节点不是详细剧情梗概，而是“结局引力场”的骨架，要足够具体，能为后续 FateWeaver 提供持续牵引。
- 节点之间应体现递进关系：前期埋入牵引，中期扩大代价，后期逼近不可回避的抉择与收束。
- 至少包含：
  - 一个会重新定义主角自我认知或身份位置的节点。
  - 一个让世界主冲突彻底升级、无法继续回避的节点。
  - 一个具有明确终局气息的收束节点。

输出要求：
1. 只输出一个合法 JSON 对象。
2. 不要输出代码块，不要输出额外标题、解释、分析、注释或对象外文本。
3. 三个字段都必须是非空字符串。
4. `keyStoryBeats` 必须是多行字符串，且每行都是一个独立节点。
5. 两段正文都要具体、克制、可演绎，避免空话、套话和泛泛而谈。"#,
        ),
        Message::user(request.prompt),
    ];

    let mut stream = std::pin::pin!(call_model(&model, &messages, None));

    while let Some(event) = stream.next().await {
        match event {
            CallModelEvent::Completed { content, .. } => {
                let data = parse_generated_profiles(&content).map_err(|message| {
                    AppError::internal(format!("模型返回格式不符合预期：{message}"))
                })?;
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

pub async fn clone_game_session(
    State(state): State<AppState>,
    Path(path): Path<SessionPath>,
) -> ApiResult<GameSessionWorldStateData> {
    let state_view = state.clone_game_session(&path.session_id).await?;
    Ok(Json(ApiResponse::ok(state_view)))
}

pub async fn generate_story_summary(
    State(state): State<AppState>,
    Path(path): Path<SessionPath>,
) -> ApiResult<StorySummaryData> {
    let narrations = state.get_game_session_narrations(&path.session_id).await?;
    if narrations.is_empty() {
        return Err(AppError::bad_request(
            "当前会话还没有可用于摘要的 narration 文本。",
        ));
    }

    let mut model = build_chat_model();
    model.set_output_json(true);

    let messages = vec![
        Message::system(
            r#"你是一名互动叙事编辑，负责把多段游戏 narration 原文整理成一段更适合展示给玩家阅读的故事摘要文案。

请严格遵守以下规则：
1. 只能依据用户提供的 narration 原文总结，不得编造原文中不存在的人物、事件、结局或设定。
2. 输出目标是一段连贯、好读、有吸引力的中文摘要文案，不是提纲、时间线、分析报告，也不是旁白续写。
3. 需要保留故事中的核心冲突、氛围变化、关键推进与悬念，但不要逐句复述。
4. 语气偏文学化、具画面感，适合放在 UI 中给玩家快速回顾剧情。
5. 摘要应聚焦“已经发生了什么、局势如何变化、人物正被什么逼近”，长度控制在 120 到 220 字。
6. 如果原文信息有限，就做克制概括，不要为了凑长度扩写。

请按以下 JSON 结构输出，字段名必须完全一致：
{
  "summary": "摘要文案"
}

输出要求：
1. 只输出一个合法 JSON 对象。
2. 不要输出代码块、解释、标题或对象外文本。
3. `summary` 必须是非空字符串。"#,
        ),
        Message::user(format!(
            "请基于以下 narration 原文生成摘要：\n\n{}",
            format_story_narrations(&narrations)
        )),
    ];

    let mut stream = std::pin::pin!(call_model(&model, &messages, None));

    while let Some(event) = stream.next().await {
        match event {
            CallModelEvent::Completed { content, .. } => {
                let summary = parse_story_summary(&content).map_err(|message| {
                    AppError::internal(format!("模型返回的摘要格式不符合预期：{message}"))
                })?;
                return Ok(Json(ApiResponse::ok(StorySummaryData {
                    summary,
                    narration_count: narrations.len(),
                })));
            }
            CallModelEvent::Error(message) => {
                return Err(AppError::internal(format!("生成故事摘要失败：{message}")));
            }
            CallModelEvent::TextChunk(_) | CallModelEvent::ReasoningChunk(_) => {}
        }
    }

    Err(AppError::internal("模型未返回完整摘要结果。"))
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
        chunk_kind: update.chunk_kind,
        chunk: update.chunk,
        output: update.output,
        error: update.error,
    }
}

fn parse_generated_profiles(content: &str) -> Result<GenerateProfilesData, String> {
    let parsed = parse_json_response::<GenerateProfilesData>(content)?;
    validate_generated_profiles(parsed)
}

fn validate_generated_profiles(data: GenerateProfilesData) -> Result<GenerateProfilesData, String> {
    let world = data.world.trim();
    if world.is_empty() {
        return Err("`world` 不能为空。".to_string());
    }

    let protagonist = data.protagonist.trim();
    if protagonist.is_empty() {
        return Err("`protagonist` 不能为空。".to_string());
    }

    let beats = data.key_story_beats.trim();
    if beats.is_empty() {
        return Err("`keyStoryBeats` 不能为空。".to_string());
    }

    Ok(GenerateProfilesData {
        world: world.to_string(),
        protagonist: protagonist.to_string(),
        key_story_beats: beats.to_string(),
    })
}

fn format_story_narrations(narrations: &[String]) -> String {
    narrations
        .iter()
        .enumerate()
        .map(|(index, text)| format!("第{}段\n{}", index + 1, text.trim()))
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn parse_story_summary(content: &str) -> Result<String, String> {
    let parsed = parse_json_response::<StorySummaryPayload>(content)?;
    validate_story_summary(parsed)
}

fn validate_story_summary(payload: StorySummaryPayload) -> Result<String, String> {
    let summary = payload.summary.trim();
    if summary.is_empty() {
        return Err("`summary` 不能为空。".to_string());
    }

    Ok(summary.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_story_narrations_numbers_and_trims_segments() {
        let formatted = format_story_narrations(&[
            "  雨水从钟楼裂隙里渗下来。 ".to_string(),
            "她听见地下回廊传来迟缓脚步。".to_string(),
        ]);

        assert_eq!(
            formatted,
            "第1段\n雨水从钟楼裂隙里渗下来。\n\n第2段\n她听见地下回廊传来迟缓脚步。"
        );
    }

    #[test]
    fn parse_story_summary_rejects_empty_summary() {
        let error = parse_story_summary(r#"{"summary":"   "}"#).expect_err("summary should fail");

        assert_eq!(error, "`summary` 不能为空。");
    }
}
