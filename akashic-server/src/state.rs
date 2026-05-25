use std::{collections::HashMap, sync::Arc};

use akashic_ecs::{
    engine::{AkashicSessionEngine, Session},
    resources::{
        export::TaskEvent,
        protagonist_action::PlayerActionType,
        task_manager::TaskUpdate,
        turn_state::TurnPhase,
    },
};
use chrono::Utc;
use tokio::sync::{Mutex, broadcast};
use uuid::Uuid;

use crate::{
    api::dto::{
        ControlGameSessionData, ControlGameSessionRequest, CreateGameSessionData,
        CreateGameSessionRequest, GameSessionControlCommand, GameSessionWorldStateData,
        SessionActionInput,
    },
    error::AppError,
};

#[derive(Clone, Default)]
pub struct AppState {
    sessions: Arc<Mutex<HashMap<String, SessionRecord>>>,
}

struct SessionRecord {
    session_id: String,
    engine: AkashicSessionEngine,
    events_tx: broadcast::Sender<TaskUpdate>,
}

pub struct LiveSessionStream {
    pub session_id: String,
    pub event_rx: broadcast::Receiver<TaskUpdate>,
}

const EVENT_CHANNEL_CAPACITY: usize = 256;

impl AppState {
    // 创建会话
    pub async fn create_game_session(
        &self,
        request: CreateGameSessionRequest,
    ) -> Result<CreateGameSessionData, AppError> {
        let session_id = format!("session-{}", Uuid::new_v4().simple());
        let created_at = now_string();
        let world_profile = request.world_profile.trim();
        let protagonist_profile = request.protagonist_profile.trim();
        if world_profile.is_empty() || protagonist_profile.is_empty() {
            return Err(AppError::bad_request("`worldProfile` 与 `protagonistProfile` 不能为空。"));
        }
        let engine = AkashicSessionEngine::new_with_profiles(world_profile, protagonist_profile);
        let mut event_rx = engine.subscribe_events();
        let (events_tx, _) = broadcast::channel(EVENT_CHANNEL_CAPACITY);
        let events_tx_for_task = events_tx.clone();
        tokio::spawn(async move {
            while let Ok(event) = event_rx.recv().await {
                let TaskEvent::TaskUpdated { update } = event;
                let _ = events_tx_for_task.send(update);
            }
        });
        let session = SessionRecord {
            session_id: session_id.clone(),
            engine,
            events_tx,
        };

        self.sessions
            .lock()
            .await
            .insert(session_id.clone(), session);
        Ok(CreateGameSessionData {
            session_id,
            created_at,
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

        match (request.control, request.action) {
            (Some(control), None) => apply_control(session, control),
            (None, Some(action)) => apply_action(session, action),
            (None, None) => Err(AppError::bad_request(
                "请求体至少需要提供 `control` 或 `action` 之一。",
            )),
            (Some(_), Some(_)) => Err(AppError::bad_request(
                "同一次请求只能执行一种操作：控制命令或玩家行动。",
            )),
        }
    }

    pub async fn open_game_session_stream(
        &self,
        session_id: &str,
        _since: Option<u64>,
    ) -> Result<LiveSessionStream, AppError> {
        let mut sessions = self.sessions.lock().await;
        let session = sessions
            .get_mut(session_id)
            .ok_or_else(|| AppError::not_found(format!("未找到会话 `{session_id}`")))?;
        Ok(LiveSessionStream {
            session_id: session_id.to_string(),
            event_rx: session.events_tx.subscribe(),
        })
    }
}

fn apply_control(
    session: &mut SessionRecord,
    control: GameSessionControlCommand,
) -> Result<ControlGameSessionData, AppError> {
    match control {
        GameSessionControlCommand::Continue => {
            session
                .engine
                .start_next_turn()
                .map_err(AppError::bad_request)?;
            Ok(ControlGameSessionData {
                action: "continue".to_string(),
            })
        }
    }
}

fn apply_action(
    session: &mut SessionRecord,
    action: SessionActionInput,
) -> Result<ControlGameSessionData, AppError> {
    let selected_action = action.action.trim();
    if selected_action.is_empty() {
        return Err(AppError::bad_request("提交行动不能为空。"));
    }

    if action.r#type == PlayerActionType::SelectedOption {
        let snapshot = session.engine.get_game_session();
        let is_valid_option = snapshot
            .choices
            .iter()
            .any(|choice| choice.option.action == selected_action);
        if !is_valid_option {
            return Err(AppError::bad_request("当前所选行动不在候选列表中。"));
        }
    }

    session
        .engine
        .submit_player_action(SessionActionInput {
            r#type: action.r#type,
            action: selected_action.to_string(),
        })
        .map_err(AppError::bad_request)?;
    session
        .engine
        .start_next_turn()
        .map_err(AppError::bad_request)?;

    Ok(ControlGameSessionData {
        action: "submit_action".to_string(),
    })
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
        current_protagonist_action: current_protagonist_action(snapshot),
        choices: snapshot.choices.clone(),
    }
}

fn latest_narration(snapshot: &Session) -> String {
    if snapshot.latest_narration.trim().is_empty() {
        snapshot.world_snapshot.description.clone()
    } else {
        snapshot.latest_narration.clone()
    }
}

fn current_protagonist_action(snapshot: &Session) -> String {
    if snapshot.current_protagonist_action.trim().is_empty() {
        "尚未做出选择".to_string()
    } else {
        snapshot.current_protagonist_action.clone()
    }
}

fn visible_turn_index(snapshot: &Session) -> u64 {
    if snapshot.phase == TurnPhase::TurnComplete {
        snapshot.turn_index.max(1)
    } else {
        snapshot.active_turn_id.max(snapshot.turn_index + 1)
    }
}

fn status_from_phase(phase: TurnPhase) -> &'static str {
    match phase {
        TurnPhase::Idle => "pending",
        TurnPhase::AwaitingPlayerChoice => "awaiting_player_choice",
        TurnPhase::TurnComplete => "waiting_control",
        TurnPhase::Failed => "failed",
        _ => "running",
    }
}

fn now_string() -> String {
    Utc::now().to_rfc3339()
}
