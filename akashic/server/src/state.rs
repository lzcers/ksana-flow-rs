use std::{collections::HashMap, sync::Arc};

use akashic_ecs::{
    engine::{AkashicSessionEngine, Session},
    resources::{
        export::TaskEvent, protagonist_action::PlayerActionType, task_manager::TaskUpdate,
        turn_state::TurnPhase,
    },
};
use chrono::Utc;
use tokio::sync::{Mutex, broadcast};
use uuid::Uuid;

use crate::{
    api::archive,
    api::dto::{
        ControlGameSessionData, ControlGameSessionRequest, CreateGameSessionData,
        CreateGameSessionRequest, CreateSaveSlotData, GameSessionControlCommand,
        GameSessionWorldStateData, RoundHistoryData, SaveExportData, SessionActionInput,
        WorldStateData,
    },
    db::ArchiveRepository,
    error::AppError,
};

#[derive(Clone)]
pub struct AppState {
    sessions: Arc<Mutex<HashMap<String, SessionRecord>>>,
    archive_repo: ArchiveRepository,
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
    pub fn new(archive_repo: ArchiveRepository) -> Result<Self, AppError> {
        archive_repo
            .init_schema()
            .map_err(|err| AppError::internal(format!("初始化存档数据库失败：{err:#}")))?;
        Ok(Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
            archive_repo,
        })
    }

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
            return Err(AppError::bad_request(
                "`worldProfile` 与 `protagonistProfile` 不能为空。",
            ));
        }
        let engine = AkashicSessionEngine::new_with_profiles(world_profile, protagonist_profile);
        let session = build_session_record(session_id.clone(), engine);

        self.sessions
            .lock()
            .await
            .insert(session_id.clone(), session);
        Ok(CreateGameSessionData {
            session_id,
            created_at,
        })
    }

    pub async fn load_game_session_from_slot(
        &self,
        slot_id: &str,
    ) -> Result<GameSessionWorldStateData, AppError> {
        let payload = self
            .archive_repo
            .load_archive_payload(slot_id)
            .map_err(|err| AppError::internal(format!("读取存档 `{slot_id}` 失败：{err:#}")))?
            .ok_or_else(|| AppError::not_found(format!("未找到存档槽 `{slot_id}`")))?;
        let session_id = payload.session_id.clone();
        let engine = archive::load_archive_payload(payload).map_err(AppError::internal)?;
        engine
            .wait_until_ready()
            .await
            .map_err(AppError::internal)?;
        let session = build_session_record(session_id.clone(), engine);
        let snapshot = session.engine.get_game_session();
        let state_view = world_state_from_session(&session, &snapshot);

        self.sessions.lock().await.insert(session_id, session);

        Ok(state_view)
    }

    pub async fn create_save_slot(
        &self,
        session_id: &str,
        title: Option<&str>,
    ) -> Result<CreateSaveSlotData, AppError> {
        let payload = {
            let mut sessions = self.sessions.lock().await;
            let session = sessions
                .get_mut(session_id)
                .ok_or_else(|| AppError::not_found(format!("未找到会话 `{session_id}`")))?;
            archive::gen_archive_payload(session_id, title, &session.engine)
                .await
                .map_err(AppError::bad_request)?
        };

        let slot_id = format!("slot-{}", Uuid::new_v4().simple());
        let saved = self
            .archive_repo
            .upsert_save_slot(&slot_id, &payload)
            .map_err(|err| AppError::internal(format!("创建存档失败：{err:#}")))?;

        Ok(CreateSaveSlotData {
            slot_id: saved.slot_id,
            session_id: saved.session_id,
            title: saved.title,
            created_at: saved.created_at,
            updated_at: saved.updated_at,
        })
    }

    pub async fn export_save_archive(
        &self,
        session_id: &str,
        title: Option<&str>,
    ) -> Result<SaveExportData, AppError> {
        let archive = {
            let mut sessions = self.sessions.lock().await;
            let session = sessions
                .get_mut(session_id)
                .ok_or_else(|| AppError::not_found(format!("未找到会话 `{session_id}`")))?;
            archive::gen_archive_payload(session_id, title, &session.engine)
                .await
                .map_err(AppError::bad_request)?
        };
        let exported_at = now_string();

        Ok(SaveExportData {
            session_id: archive.session_id.clone(),
            title: archive.title.clone(),
            created_at: exported_at.clone(),
            updated_at: exported_at,
            archive,
        })
    }

    pub async fn load_game_session_from_archive(
        &self,
        payload: archive::SessionArchivePayload,
    ) -> Result<GameSessionWorldStateData, AppError> {
        archive::validate_archive_payload(&payload).map_err(AppError::bad_request)?;
        let session_id = payload.session_id.clone();
        let engine = archive::load_archive_payload(payload).map_err(AppError::bad_request)?;
        engine
            .wait_until_ready()
            .await
            .map_err(AppError::internal)?;
        let session = build_session_record(session_id.clone(), engine);
        let snapshot = session.engine.get_game_session();
        let state_view = world_state_from_session(&session, &snapshot);

        self.sessions.lock().await.insert(session_id, session);

        Ok(state_view)
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

fn build_session_record(session_id: String, engine: AkashicSessionEngine) -> SessionRecord {
    let mut event_rx = engine.subscribe_events();
    let (events_tx, _) = broadcast::channel(EVENT_CHANNEL_CAPACITY);
    let events_tx_for_task = events_tx.clone();
    tokio::spawn(async move {
        while let Ok(event) = event_rx.recv().await {
            let TaskEvent::TaskUpdated { update } = event;
            let _ = events_tx_for_task.send(update);
        }
    });

    SessionRecord {
        session_id,
        engine,
        events_tx,
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
        world_state: WorldStateData::from(snapshot.world_snapshot.clone()),
        history: snapshot
            .history
            .clone()
            .into_iter()
            .map(RoundHistoryData::from)
            .collect(),
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

#[cfg(test)]
mod tests {
    use std::fs;

    use agent::agent::context::Context;
    use akashic_ecs::resources::{
        history::{RoundHistoryEntry, SessionHistoryLog},
        protagonist_action::{PendingProtagonistChoice, ProtagonistOption},
        turn_state::TurnPhase,
        world_snapshot::WorldSnapshot,
    };
    use serde_json::Value;
    use uuid::Uuid;

    use super::*;
    use crate::api::archive::{
        ProtagonistDecisionArchive, SessionArchivePayload, TurnStateArchive,
    };

    #[tokio::test]
    async fn load_game_session_from_slot_restores_runtime_into_registry() {
        let db_path = std::env::temp_dir().join(format!("state-load-{}.sqlite3", Uuid::new_v4()));
        let repo = ArchiveRepository::new(&db_path);
        repo.init_schema().expect("schema should initialize");
        repo.upsert_save_slot("slot-load", &sample_payload())
            .expect("payload should persist");

        let state = AppState::new(repo).expect("app state should initialize");
        let restored = state
            .load_game_session_from_slot("slot-load")
            .await
            .expect("slot should restore");

        assert_eq!(restored.session_id, "session-from-slot");
        assert_eq!(restored.phase, TurnPhase::AwaitingPlayerChoice);
        assert_eq!(restored.turn_index, 8);
        assert_eq!(restored.active_turn_id, 7);
        assert_eq!(restored.current_protagonist_action, "绕到钟楼背面");
        assert_eq!(restored.choices.len(), 1);
        assert_eq!(restored.history.len(), 1);
        assert_eq!(restored.history[0].narration_text, "钟声掠过雾墙。");
        assert_eq!(
            restored.history[0].selected_choice_text.as_deref(),
            Some("绕行")
        );

        let loaded_again = state
            .get_game_session_world("session-from-slot")
            .await
            .expect("restored session should be queryable");
        assert_eq!(loaded_again.world_state.scene_title, "钟楼阴影");

        let _ = fs::remove_file(db_path);
    }

    #[tokio::test]
    async fn create_save_slot_persists_archive_payload() {
        let db_path = std::env::temp_dir().join(format!("state-save-{}.sqlite3", Uuid::new_v4()));
        let repo = ArchiveRepository::new(&db_path);
        let state = AppState::new(repo.clone()).expect("app state should initialize");

        let created = state
            .create_game_session(crate::api::dto::CreateGameSessionRequest {
                world_profile: "archive world".to_string(),
                protagonist_profile: "archive protagonist".to_string(),
            })
            .await
            .expect("session should create");

        let saved = state
            .create_save_slot(&created.session_id, Some("测试存档"))
            .await
            .expect("save slot should be created");

        let persisted = repo
            .load_archive_payload(&saved.slot_id)
            .expect("db query should succeed")
            .expect("payload should exist");

        assert_eq!(persisted.session_id, created.session_id);
        assert_eq!(persisted.title, "测试存档");
        assert_eq!(saved.title, "测试存档");

        let _ = fs::remove_file(db_path);
    }

    #[tokio::test]
    async fn load_game_session_from_archive_overwrites_existing_session() {
        let db_path =
            std::env::temp_dir().join(format!("state-archive-{}.sqlite3", Uuid::new_v4()));
        let repo = ArchiveRepository::new(&db_path);
        let state = AppState::new(repo).expect("app state should initialize");

        state
            .create_game_session(crate::api::dto::CreateGameSessionRequest {
                world_profile: "old world".to_string(),
                protagonist_profile: "old protagonist".to_string(),
            })
            .await
            .expect("session should create");

        let mut payload = sample_payload();
        payload.session_id = "session-b".to_string();
        let restored = state
            .load_game_session_from_archive(payload.clone())
            .await
            .expect("archive should restore");

        assert_eq!(restored.session_id, "session-b");
        assert_eq!(restored.world_state.scene_title, "钟楼阴影");

        let loaded_again = state
            .get_game_session_world("session-b")
            .await
            .expect("restored session should be queryable");
        assert_eq!(loaded_again.current_protagonist_action, "绕到钟楼背面");

        let _ = fs::remove_file(db_path);
    }

    #[test]
    fn game_session_world_state_serializes_world_state_as_camel_case() {
        let dto = GameSessionWorldStateData {
            session_id: "session-test".to_string(),
            status: "awaiting_player_choice".to_string(),
            phase: TurnPhase::AwaitingPlayerChoice,
            turn_index: 2,
            active_turn_id: 2,
            world_state: WorldStateData::from(WorldSnapshot {
                round: 2,
                scene_title: "螺旋楼梯的暗影".to_string(),
                time_absolute: "第一日 深夜十一点四十二分".to_string(),
                location_name: "齿轮教堂地下二层".to_string(),
                new_info: vec!["图纸碎片已安全到手".to_string()],
                ..WorldSnapshot::default()
            }),
            history: vec![],
            current_task: None,
            tasks: vec![],
            latest_narration: "narration".to_string(),
            current_protagonist_action: "action".to_string(),
            choices: vec![],
        };

        let value = serde_json::to_value(dto).expect("dto should serialize");
        let world_state = value
            .get("worldState")
            .and_then(Value::as_object)
            .expect("worldState should be serialized as object");

        assert_eq!(
            world_state.get("sceneTitle").and_then(Value::as_str),
            Some("螺旋楼梯的暗影")
        );
        assert_eq!(
            world_state.get("timeAbsolute").and_then(Value::as_str),
            Some("第一日 深夜十一点四十二分")
        );
        assert_eq!(
            world_state.get("locationName").and_then(Value::as_str),
            Some("齿轮教堂地下二层")
        );
        assert_eq!(
            world_state
                .get("newInfo")
                .and_then(Value::as_array)
                .and_then(|items| items.first())
                .and_then(Value::as_str),
            Some("图纸碎片已安全到手")
        );
        assert!(world_state.get("scene_title").is_none());
        assert!(world_state.get("new_info").is_none());
    }

    fn sample_payload() -> SessionArchivePayload {
        SessionArchivePayload {
            session_id: "session-from-slot".to_string(),
            title: "第7轮：钟楼阴影".to_string(),
            world_profile: "world".to_string(),
            protagonist_profile: "protagonist".to_string(),
            turn_state: TurnStateArchive {
                phase: TurnPhase::AwaitingPlayerChoice,
                turn_index: 7,
                active_turn_id: 7,
            },
            fate_weaver: Context::default(),
            upper_narrator: Context::default(),
            protagonist: Context::default(),
            world_snapshot: WorldSnapshot {
                round: 7,
                scene_title: "钟楼阴影".to_string(),
                description: "雾气正在台阶间倒灌。".to_string(),
                ..WorldSnapshot::default()
            },
            protagonist_decision: ProtagonistDecisionArchive {
                committed_action: "绕到钟楼背面".to_string(),
                choices: vec![PendingProtagonistChoice {
                    id: "choice-1".to_string(),
                    option: ProtagonistOption {
                        title: "绕行".to_string(),
                        action: "绕到钟楼背面".to_string(),
                        motivation_and_risk: "视野更好，但会暴露脚步声".to_string(),
                    },
                }],
            },
            history_log: SessionHistoryLog {
                rounds: vec![RoundHistoryEntry {
                    round: 7,
                    world_snapshot: Some(WorldSnapshot {
                        round: 7,
                        scene_title: "钟楼阴影".to_string(),
                        description: "雾气正在台阶间倒灌。".to_string(),
                        ..WorldSnapshot::default()
                    }),
                    narration_text: Some("钟声掠过雾墙。".to_string()),
                    choices: vec![PendingProtagonistChoice {
                        id: "choice-1".to_string(),
                        option: ProtagonistOption {
                            title: "绕行".to_string(),
                            action: "绕到钟楼背面".to_string(),
                            motivation_and_risk: "视野更好，但会暴露脚步声".to_string(),
                        },
                    }],
                    committed_action: Some("绕到钟楼背面".to_string()),
                }],
            },
        }
    }
}
