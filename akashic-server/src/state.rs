use std::{collections::HashMap, sync::Arc};

use akashic_ecs::{
    engine::{AkashicSessionEngine, Session},
    resources::{
        export::{TaskEvent, TaskView},
        turn_state::TurnPhase,
    },
};
use chrono::Utc;
use tokio::sync::{Mutex, broadcast};
use uuid::Uuid;

use crate::{
    api::dto::{
        Character, ControlGameSessionData, ControlGameSessionRequest, CreateGameSessionData,
        CreateGameSessionRequest, GameSessionControlCommand, GameSessionWorldStateData,
        HistoryItem, SessionChoiceInput, World,
    },
    error::AppError,
};

#[derive(Clone, Default)]
pub struct AppState {
    sessions: Arc<Mutex<HashMap<String, SessionRecord>>>,
}

struct SessionRecord {
    session_id: String,
    history: Vec<HistoryItem>,
    last_selected_choice: Option<String>,
    engine: AkashicSessionEngine,
}

pub struct LiveSessionStream {
    pub session_id: String,
    pub tasks: Vec<TaskView>,
    pub event_rx: broadcast::Receiver<TaskEvent>,
}

impl AppState {
    pub async fn create_game_session(
        &self,
        request: CreateGameSessionRequest,
    ) -> Result<CreateGameSessionData, AppError> {
        let session_id = format!("session-{}", Uuid::new_v4().simple());
        let created_at = now_string();
        let protagonist_profile = build_protagonist_profile(&request.character);
        let world_profile = build_world_profile(&request.world);
        let engine = AkashicSessionEngine::new_with_profiles(&world_profile, &protagonist_profile);
        let session = SessionRecord {
            session_id: session_id.clone(),
            history: Vec::new(),
            last_selected_choice: None,
            engine,
        };

        self.sessions
            .lock()
            .await
            .insert(session_id.clone(), session);
        Ok(CreateGameSessionData {
            session_id,
            created_at,
            status: "pending".to_string(),
        })
    }

    pub async fn get_game_session_world(
        &self,
        session_id: &str,
    ) -> Result<GameSessionWorldStateData, AppError> {
        let mut sessions = self.sessions.lock().await;
        let session = sessions
            .get_mut(session_id)
            .ok_or_else(|| AppError::not_found(format!("未找到会话 `{session_id}`")))?;

        ensure_session_started(session).await?;
        let snapshot = session.engine.get_game_session();

        Ok(world_state_from_session(session, &snapshot))
    }

    pub async fn control_game_session(
        &self,
        session_id: &str,
        request: ControlGameSessionRequest,
    ) -> Result<ControlGameSessionData, AppError> {
        let mut sessions = self.sessions.lock().await;
        let session = sessions
            .get_mut(session_id)
            .ok_or_else(|| AppError::not_found(format!("未找到会话 `{session_id}`")))?;

        match (request.control, request.choice) {
            (Some(control), None) => apply_control(session, control).await,
            (None, Some(choice)) => apply_choice(session, choice).await,
            (None, None) => Err(AppError::bad_request(
                "请求体至少需要提供 `control` 或 `choice` 之一。",
            )),
            (Some(_), Some(_)) => Err(AppError::bad_request(
                "同一次请求只能执行一种操作：控制命令或玩家选择。",
            )),
        }
    }

    pub async fn open_game_session_stream(
        &self,
        session_id: &str,
    ) -> Result<LiveSessionStream, AppError> {
        let mut sessions = self.sessions.lock().await;
        let session = sessions
            .get_mut(session_id)
            .ok_or_else(|| AppError::not_found(format!("未找到会话 `{session_id}`")))?;

        let snapshot = session.engine.get_game_session();

        Ok(LiveSessionStream {
            session_id: session_id.to_string(),
            tasks: snapshot.tasks.clone(),
            event_rx: session.engine.subscribe_events(),
        })
    }

    pub async fn ensure_game_session_started(&self, session_id: &str) -> Result<(), AppError> {
        let mut sessions = self.sessions.lock().await;
        let session = sessions
            .get_mut(session_id)
            .ok_or_else(|| AppError::not_found(format!("未找到会话 `{session_id}`")))?;

        ensure_session_started(session).await
    }
}

async fn apply_control(
    session: &mut SessionRecord,
    control: GameSessionControlCommand,
) -> Result<ControlGameSessionData, AppError> {
    match control {
        GameSessionControlCommand::Continue => {
            let snapshot = session
                .engine
                .continue_turn()
                .await
                .map_err(AppError::bad_request)?;
            push_narration_history(session, &snapshot);
            Ok(ControlGameSessionData {
                action: "continue".to_string(),
                session: world_state_from_session(session, &snapshot),
            })
        }
    }
}

async fn apply_choice(
    session: &mut SessionRecord,
    choice: SessionChoiceInput,
) -> Result<ControlGameSessionData, AppError> {
    ensure_session_started(session).await?;
    let current = session.engine.get_game_session();
    let selected_choice = current
        .choices
        .iter()
        .find(|item| item.id == choice.choice_id)
        .cloned()
        .ok_or_else(|| AppError::bad_request("所选分支不存在或已失效。"))?;
    let selected_text = if selected_choice.option.title.is_empty() {
        selected_choice.option.action
    } else {
        selected_choice.option.title
    };

    session.last_selected_choice = Some(selected_text.clone());
    session.history.push(HistoryItem {
        item_type: "choice".to_string(),
        turn_index: visible_turn_index(&current),
        text: selected_text,
        created_at: now_string(),
    });

    let snapshot = session
        .engine
        .submit_choice(&choice.choice_id)
        .await
        .map_err(AppError::bad_request)?;
    push_narration_history(session, &snapshot);

    Ok(ControlGameSessionData {
        action: "submit_choice".to_string(),
        session: world_state_from_session(session, &snapshot),
    })
}

async fn ensure_session_started(session: &mut SessionRecord) -> Result<(), AppError> {
    let current = session.engine.get_game_session();
    if current.phase != TurnPhase::Idle {
        return Ok(());
    }

    let snapshot = session
        .engine
        .continue_turn()
        .await
        .map_err(AppError::bad_request)?;
    push_narration_history(session, &snapshot);

    Ok(())
}

fn world_state_from_session(
    session: &SessionRecord,
    snapshot: &Session,
) -> GameSessionWorldStateData {
    GameSessionWorldStateData {
        session_id: session.session_id.clone(),
        status: status_from_phase(snapshot.phase).to_string(),
        phase: snapshot.phase,
        turn_index: visible_turn_index(snapshot),
        active_turn_id: snapshot.active_turn_id,
        world_state: snapshot.world_snapshot.clone(),
        current_task: snapshot.current_task.clone(),
        tasks: snapshot.tasks.clone(),
        latest_narration: latest_narration(snapshot),
        current_protagonist_action: current_protagonist_action(session, snapshot),
        choices: snapshot.choices.clone(),
    }
}

fn push_narration_history(session: &mut SessionRecord, snapshot: &Session) {
    let text = latest_narration(snapshot);
    if text.trim().is_empty() {
        return;
    }

    let turn_index = visible_turn_index(snapshot);
    let duplicated = session.history.last().is_some_and(|item| {
        item.item_type == "narration" && item.turn_index == turn_index && item.text == text
    });
    if duplicated {
        return;
    }

    session.history.push(HistoryItem {
        item_type: "narration".to_string(),
        turn_index,
        text,
        created_at: now_string(),
    });
}

fn latest_narration(snapshot: &Session) -> String {
    if snapshot.latest_narration.trim().is_empty() {
        snapshot.world_snapshot.description.clone()
    } else {
        snapshot.latest_narration.clone()
    }
}

fn current_protagonist_action(session: &SessionRecord, snapshot: &Session) -> String {
    if snapshot.current_protagonist_action.trim().is_empty() {
        session
            .last_selected_choice
            .clone()
            .unwrap_or_else(|| "尚未做出选择".to_string())
    } else {
        snapshot.current_protagonist_action.clone()
    }
}

fn visible_turn_index(snapshot: &Session) -> u64 {
    if snapshot.phase == TurnPhase::TurnFinished {
        snapshot.turn_index.max(1)
    } else {
        snapshot.turn_index + 1
    }
}

fn status_from_phase(phase: TurnPhase) -> &'static str {
    match phase {
        TurnPhase::Idle => "pending",
        TurnPhase::AwaitingPlayerChoice => "awaiting_player_choice",
        TurnPhase::TurnFinished => "waiting_control",
        TurnPhase::Failed => "failed",
        _ => "running",
    }
}

fn build_protagonist_profile(character: &Character) -> String {
    format!(
        "## 人物设定\n- 姓名：{}\n- 性别：{}\n- 年龄：{}\n- 外貌：{}\n- 背景：{}\n- 性格倾向：勇气 {} / 理性 {} / 利他 {}\n",
        character.name,
        character.gender,
        character.age,
        character.appearance,
        character.background,
        character.traits.courage,
        character.traits.rationality,
        character.traits.altruism,
    )
}

fn build_world_profile(world: &World) -> String {
    format!(
        "## 世界设定\n- 时代：{}\n- 核心冲突：{}\n- 特殊规则：{}\n",
        world.era,
        world.core_conflict,
        if world.special_rules.is_empty() {
            "无".to_string()
        } else {
            world.special_rules.join("；")
        }
    )
}

fn now_string() -> String {
    Utc::now().to_rfc3339()
}
