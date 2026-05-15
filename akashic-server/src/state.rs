use std::{collections::HashMap, sync::Arc};

use akashic_ecs::{
    engine::{AkashicSessionEngine, Session},
    resources::export::SessionEvent,
    resources::turn_state::TurnPhase,
    resources::world_snapshot::WorldSnapshot,
};
use chrono::Utc;
use tokio::sync::{Mutex, broadcast};
use uuid::Uuid;

use crate::{
    api::dto::{
        Character, Choice, ChoiceCostHints, CreateGameSessionData, CreateGameSessionRequest,
        EndingData, EndingTurningPoint, GameSessionEndingData, GameSessionSnapshot, HistoryItem,
        HistoryListData, ResourceDelta, RuntimeStateView, SessionResources, StoryNode,
        StreamHandshakeData, SubmitChoiceData, SubmitChoiceRequest, World,
    },
    error::AppError,
};

#[derive(Clone, Default)]
pub struct AppState {
    sessions: Arc<Mutex<HashMap<String, SessionRecord>>>,
}

struct SessionRecord {
    session_id: String,
    status: String,
    character: Character,
    world: World,
    resources: SessionResources,
    current_node: StoryNode,
    state_view: RuntimeStateView,
    ending_status: String,
    ending: Option<EndingData>,
    history: Vec<HistoryItem>,
    last_selected_choice: Option<String>,
    engine: AkashicSessionEngine,
}

pub struct LiveSessionStream {
    pub handshake: StreamHandshakeData,
    pub snapshot: GameSessionSnapshot,
    pub history: HistoryListData,
    pub ending: Option<GameSessionEndingData>,
    pub event_rx: broadcast::Receiver<SessionEvent>,
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
        let mut engine =
            AkashicSessionEngine::new_with_profiles(&world_profile, &protagonist_profile);
        let snapshot = engine
            .continue_turn()
            .await
            .map_err(AppError::bad_request)?;
        let resources = initial_resources(&request.world, &snapshot);

        let mut session = SessionRecord {
            session_id: session_id.clone(),
            status: "active".to_string(),
            character: request.character.clone(),
            world: request.world.clone(),
            resources,
            current_node: empty_story_node(&request.world.era),
            state_view: empty_state_view(),
            ending_status: "pending".to_string(),
            ending: None,
            history: Vec::new(),
            last_selected_choice: None,
            engine,
        };
        apply_engine_snapshot(&mut session, &snapshot);
        session.history.push(HistoryItem {
            item_type: "narration".to_string(),
            turn_index: session.state_view.turn_index,
            text: session.current_node.text.clone(),
            created_at: created_at.clone(),
        });

        let response = GameSessionSnapshot {
            session_id: session.session_id.clone(),
            status: session.status.clone(),
            character: session.character.clone(),
            world: session.world.clone(),
            resources: session.resources.clone(),
            current_node: session.current_node.clone(),
            state_view: session.state_view.clone(),
            ending_status: session.ending_status.clone(),
        };

        self.sessions.lock().await.insert(session_id, session);
        Ok(CreateGameSessionData {
            session_id: response.session_id,
            created_at,
            character: response.character,
            world: response.world,
            resources: response.resources,
            current_node: response.current_node,
            state_view: response.state_view,
        })
    }

    pub async fn get_game_session(
        &self,
        session_id: &str,
    ) -> Result<GameSessionSnapshot, AppError> {
        let sessions = self.sessions.lock().await;
        let session = sessions
            .get(session_id)
            .ok_or_else(|| AppError::not_found(format!("未找到会话 `{session_id}`")))?;

        Ok(snapshot_from_session(session))
    }

    pub async fn submit_choice(
        &self,
        session_id: &str,
        request: SubmitChoiceRequest,
    ) -> Result<SubmitChoiceData, AppError> {
        let mut sessions = self.sessions.lock().await;
        let session = sessions
            .get_mut(session_id)
            .ok_or_else(|| AppError::not_found(format!("未找到会话 `{session_id}`")))?;

        if session.ending_status == "ready" {
            return Err(AppError::bad_request(
                "当前会话已经抵达结局，无法继续推进。",
            ));
        }

        let choice = session
            .current_node
            .choices
            .iter()
            .find(|item| item.id == request.choice_id)
            .cloned()
            .ok_or_else(|| AppError::bad_request("所选分支不存在或已失效。"))?;

        if request.use_obsession && session.resources.obsession_points <= 0 {
            return Err(AppError::bad_request("执念不足，无法强化这次抉择。"));
        }

        let previous_turn = session.state_view.turn_index;
        let mut resource_delta = ResourceDelta {
            obsession_points: 0,
            intuition_points: 0,
        };
        if request.use_obsession {
            session.resources.obsession_points -= 1;
            resource_delta.obsession_points = -1;
        }
        session.resources.days_left = (session.resources.days_left - 6).max(0);
        session.last_selected_choice = Some(choice.text.clone());
        session.history.push(HistoryItem {
            item_type: "choice".to_string(),
            turn_index: previous_turn,
            text: if request.use_obsession {
                format!("{}（倾注执念）", choice.text)
            } else {
                choice.text.clone()
            },
            created_at: now_string(),
        });

        let snapshot = session
            .engine
            .submit_choice(&request.choice_id)
            .await
            .map_err(AppError::bad_request)?;
        apply_engine_snapshot(session, &snapshot);

        if snapshot.phase == TurnPhase::TurnFinished {
            let ending = build_ending(session, &choice.text, request.use_obsession);
            session.status = "completed".to_string();
            session.ending_status = "ready".to_string();
            session.ending = Some(ending.clone());
            session.current_node.choices.clear();
            session.history.push(HistoryItem {
                item_type: "ending".to_string(),
                turn_index: session.state_view.turn_index,
                text: ending.legacy.clone(),
                created_at: now_string(),
            });
        } else {
            session.history.push(HistoryItem {
                item_type: "narration".to_string(),
                turn_index: session.state_view.turn_index,
                text: session.current_node.text.clone(),
                created_at: now_string(),
            });
        }

        Ok(SubmitChoiceData {
            accepted: true,
            session_id: session_id.to_string(),
            turn_id: session.state_view.active_turn_id,
            resource_delta,
            resources: session.resources.clone(),
            state_view: session.state_view.clone(),
        })
    }

    pub async fn get_game_session_ending(
        &self,
        session_id: &str,
    ) -> Result<GameSessionEndingData, AppError> {
        let sessions = self.sessions.lock().await;
        let session = sessions
            .get(session_id)
            .ok_or_else(|| AppError::not_found(format!("未找到会话 `{session_id}`")))?;
        let ending = session
            .ending
            .clone()
            .ok_or_else(|| AppError::bad_request("当前会话尚未生成结局。"))?;

        Ok(GameSessionEndingData {
            session_id: session.session_id.clone(),
            ending_status: session.ending_status.clone(),
            ending,
        })
    }

    pub async fn open_game_session_stream(
        &self,
        session_id: &str,
    ) -> Result<LiveSessionStream, AppError> {
        let sessions = self.sessions.lock().await;
        let session = sessions
            .get(session_id)
            .ok_or_else(|| AppError::not_found(format!("未找到会话 `{session_id}`")))?;

        Ok(LiveSessionStream {
            handshake: StreamHandshakeData {
                session_id: session_id.to_string(),
                protocol: "engine-sse".to_string(),
                note: "当前连接直接订阅 akashic-ecs 的会话事件流，可实时接收任务输出与状态推进。"
                    .to_string(),
            },
            snapshot: snapshot_from_session(session),
            history: HistoryListData {
                items: session.history.clone(),
            },
            ending: session.ending.clone().map(|ending| GameSessionEndingData {
                session_id: session.session_id.clone(),
                ending_status: session.ending_status.clone(),
                ending,
            }),
            event_rx: session.engine.subscribe_events(),
        })
    }
}

fn snapshot_from_session(session: &SessionRecord) -> GameSessionSnapshot {
    GameSessionSnapshot {
        session_id: session.session_id.clone(),
        status: session.status.clone(),
        character: session.character.clone(),
        world: session.world.clone(),
        resources: session.resources.clone(),
        current_node: session.current_node.clone(),
        state_view: session.state_view.clone(),
        ending_status: session.ending_status.clone(),
    }
}

fn initial_resources(world: &World, snapshot: &Session) -> SessionResources {
    SessionResources {
        obsession_points: 3,
        intuition_points: 5,
        days_left: 30,
        world_news: Some(if snapshot.world_snapshot.current_event.is_empty() {
            format!("{}的命运涟漪开始汇聚。", world.era)
        } else {
            snapshot.world_snapshot.current_event.clone()
        }),
    }
}

fn apply_engine_snapshot(session: &mut SessionRecord, snapshot: &Session) {
    let turn_finished = snapshot.phase == TurnPhase::TurnFinished;
    let display_turn_index = if turn_finished {
        snapshot.turn_index.max(1)
    } else {
        snapshot.turn_index + 1
    };
    let latest_history = if snapshot.latest_narration.is_empty() {
        snapshot.world_snapshot.description.clone()
    } else {
        snapshot.latest_narration.clone()
    };
    let latest_protagonist_action = session
        .last_selected_choice
        .clone()
        .unwrap_or_else(|| "尚未做出选择".to_string());

    session.resources.world_news = Some(if snapshot.world_snapshot.current_event.is_empty() {
        snapshot.world_snapshot.pacing_note.clone()
    } else {
        snapshot.world_snapshot.current_event.clone()
    });
    session.status = if turn_finished {
        "completed".to_string()
    } else {
        "active".to_string()
    };
    session.ending_status = if turn_finished {
        "ready".to_string()
    } else {
        "pending".to_string()
    };
    session.current_node = StoryNode {
        id: format!("node-{}", display_turn_index),
        text: latest_history.clone(),
        image: cover_image_for(&session.world.era),
        choices: if turn_finished {
            Vec::new()
        } else {
            snapshot
                .choices
                .iter()
                .map(|choice| Choice {
                    id: choice.id.clone(),
                    text: if choice.option.title.is_empty() {
                        choice.option.action.clone()
                    } else {
                        choice.option.title.clone()
                    },
                    disabled: false,
                    cost_hints: ChoiceCostHints {
                        intuition: 1,
                        obsession: 1,
                    },
                })
                .collect()
        },
    };
    session.state_view = RuntimeStateView {
        game_state: if turn_finished {
            "ending".to_string()
        } else {
            "playing".to_string()
        },
        phase: if turn_finished {
            "EndingReady".to_string()
        } else {
            "AwaitingChoice".to_string()
        },
        turn_index: display_turn_index,
        active_turn_id: snapshot.active_turn_id,
        current_location: snapshot.world_snapshot.location_name.clone(),
        current_scene: if turn_finished {
            "余响归档".to_string()
        } else {
            snapshot.world_snapshot.scene_title.clone()
        },
        protagonist_state: snapshot.world_snapshot.protagonist_condition.clone(),
        npcs_state: summarize_npcs(&snapshot.world_snapshot),
        latest_history,
        latest_broadcast_summary: snapshot.world_snapshot.current_event.clone(),
        latest_protagonist_action,
    };
}

fn summarize_npcs(snapshot: &WorldSnapshot) -> String {
    if snapshot.npcs.is_empty() {
        return "关键角色仍在观望，你的下一步会改变他们的站位。".to_string();
    }

    snapshot
        .npcs
        .iter()
        .take(2)
        .map(|npc| format!("{}：{}，{}", npc.name, npc.mood, npc.goal))
        .collect::<Vec<_>>()
        .join("；")
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

fn empty_story_node(era: &str) -> StoryNode {
    StoryNode {
        id: "node-0".to_string(),
        text: String::new(),
        image: cover_image_for(era),
        choices: Vec::new(),
    }
}

fn empty_state_view() -> RuntimeStateView {
    RuntimeStateView {
        game_state: "booting".to_string(),
        phase: "Booting".to_string(),
        turn_index: 0,
        active_turn_id: 0,
        current_location: String::new(),
        current_scene: String::new(),
        protagonist_state: String::new(),
        npcs_state: String::new(),
        latest_history: String::new(),
        latest_broadcast_summary: String::new(),
        latest_protagonist_action: String::new(),
    }
}

fn build_ending(session: &SessionRecord, choice_text: &str, used_obsession: bool) -> EndingData {
    EndingData {
        biography: format!(
            "{}的《此生回响录》：在{}的年代，你以“{}”的身份卷入“{}”的漩涡。经历三次关键抉择后，你最终选择“{}”{}，让这段人生以一种无法被轻易遗忘的方式刻进了世界的边缘。",
            session.character.name,
            session.world.era,
            session.character.background,
            session.world.core_conflict,
            choice_text,
            if used_obsession {
                "，并在最后一刻倾注了全部执念"
            } else {
                ""
            }
        ),
        turning_points: vec![
            EndingTurningPoint {
                cause: "第一道线索在雨夜中显形".to_string(),
                effect: format!("你决定不再逃避，正式踏入{}的暗流", session.world.era),
            },
            EndingTurningPoint {
                cause: "盟友与阴谋同时逼近".to_string(),
                effect: "你学会在代价和真相之间挑选真正重要的东西".to_string(),
            },
            EndingTurningPoint {
                cause: "最终抉择压向命运中央".to_string(),
                effect: if used_obsession {
                    format!("你以“{}”完成收束，并让执念化作最后的推进力", choice_text)
                } else {
                    format!("你以“{}”结束这段旅程，留下足够长久的回响", choice_text)
                },
            },
        ],
        legacy: format!(
            "精神遗产评估：{}让{}的人们记住，面对“{}”时仍然可以选择怎样活下去。",
            session.character.name, session.world.era, session.world.core_conflict
        ),
        cgs: vec![
            cover_image_for(&session.world.era),
            cover_image_for("东方玄幻"),
            cover_image_for("蒸汽朋克"),
        ],
    }
}

fn now_string() -> String {
    Utc::now().to_rfc3339()
}

fn cover_image_for(era: &str) -> String {
    match era {
        "蒸汽朋克" => "https://coresg-normal.trae.ai/api/ide/v1/text_to_image?prompt=A%20dim%20steampunk%20district%20in%20the%20rain%2C%20warm%20light%20leaking%20from%20a%20tavern%20door%2C%20moody%20cinematic%20concept%20art&image_size=landscape_16_9".to_string(),
        "星际拓荒" => "https://coresg-normal.trae.ai/api/ide/v1/text_to_image?prompt=A%20futuristic%20spaceport%20at%20night%2C%20neon%20fog%2C%20cinematic%20concept%20art&image_size=landscape_16_9".to_string(),
        "东方玄幻" => "https://coresg-normal.trae.ai/api/ide/v1/text_to_image?prompt=A%20mystical%20eastern%20city%20in%20blue%20mist%2C%20golden%20lanterns%2C%20cinematic%20concept%20art&image_size=landscape_16_9".to_string(),
        "末日废土" => "https://coresg-normal.trae.ai/api/ide/v1/text_to_image?prompt=A%20post-apocalyptic%20harbor%20at%20night%2C%20lonely%20watchman%2C%20blue%20glow%2C%20cinematic%20concept%20art&image_size=landscape_16_9".to_string(),
        _ => "https://coresg-normal.trae.ai/api/ide/v1/text_to_image?prompt=A%20mysterious%20dark%20city%20with%20golden%20lights%2C%20cinematic%20concept%20art&image_size=landscape_16_9".to_string(),
    }
}
