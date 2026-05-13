use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use akashic_ecs::{
    engine::{AkashicSessionEngine, AkashicSessionSnapshot, ChoiceResolutionMode},
    resources::world_snapshot::WorldSnapshot,
};
use chrono::Utc;
use uuid::Uuid;

use crate::{
    api::dto::{
        ArchiveDetailData, ArchiveListData, ArchiveListItem, Character, Choice, ChoiceCostHints,
        CreateGameSessionData, CreateGameSessionRequest, CreateSaveRequest, EndingData,
        EndingTurningPoint, GameSessionEndingData, GameSessionSnapshot, HistoryItem,
        HistoryListData, IntuitionPreviewData, LoadSaveData, ResourceDelta, RuntimeStateView,
        SaveListData, SaveListItem, SaveSummary, SessionResources, ShareCardData, StoryNode,
        StreamHandshakeData, SubmitChoiceData, SubmitChoiceRequest, World,
    },
    error::AppError,
};

#[derive(Clone)]
pub struct AppState {
    pub service_name: &'static str,
    pub api_version: &'static str,
    store: Arc<Mutex<MockStore>>,
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

#[derive(Debug, Clone)]
struct SaveRecord {
    summary: SaveSummary,
}

#[derive(Debug, Clone)]
struct ArchiveRecord {
    list_item: ArchiveListItem,
    detail: ArchiveDetailData,
}

struct MockStore {
    sessions: HashMap<String, SessionRecord>,
    saves: HashMap<String, SaveRecord>,
    archives: HashMap<String, ArchiveRecord>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            service_name: "akashic-server",
            api_version: "v0-draft",
            store: Arc::new(Mutex::new(MockStore::default())),
        }
    }
}

impl Default for MockStore {
    fn default() -> Self {
        let archives = seed_archives()
            .into_iter()
            .map(|archive| (archive.detail.archive_id.clone(), archive))
            .collect();

        Self {
            sessions: HashMap::new(),
            saves: HashMap::new(),
            archives,
        }
    }
}

impl AppState {
    pub fn create_game_session(
        &self,
        request: CreateGameSessionRequest,
    ) -> Result<CreateGameSessionData, AppError> {
        let session_id = format!("session-{}", Uuid::new_v4().simple());
        let created_at = now_string();
        let protagonist_profile = build_protagonist_profile(&request.character);
        let world_profile = build_world_profile(&request.world);
        let mut engine = AkashicSessionEngine::new_with_profiles_and_mode(
            &world_profile,
            &protagonist_profile,
            ChoiceResolutionMode::WaitForUser,
        );
        let snapshot = engine.bootstrap().map_err(AppError::bad_request)?;
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

        let response = CreateGameSessionData {
            session_id: session_id.clone(),
            created_at,
            character: session.character.clone(),
            world: session.world.clone(),
            resources: session.resources.clone(),
            current_node: session.current_node.clone(),
            state_view: session.state_view.clone(),
        };

        let mut store = self.store.lock().expect("mock store poisoned");
        store.sessions.insert(session_id, session);
        Ok(response)
    }

    pub fn get_game_session(&self, session_id: &str) -> Result<GameSessionSnapshot, AppError> {
        let store = self.store.lock().expect("mock store poisoned");
        let session = store
            .sessions
            .get(session_id)
            .ok_or_else(|| AppError::not_found(format!("未找到会话 `{session_id}`")))?;

        Ok(GameSessionSnapshot {
            session_id: session.session_id.clone(),
            status: session.status.clone(),
            character: session.character.clone(),
            world: session.world.clone(),
            resources: session.resources.clone(),
            current_node: session.current_node.clone(),
            state_view: session.state_view.clone(),
            ending_status: session.ending_status.clone(),
        })
    }

    pub fn submit_choice(
        &self,
        session_id: &str,
        request: SubmitChoiceRequest,
    ) -> Result<SubmitChoiceData, AppError> {
        let mut store = self.store.lock().expect("mock store poisoned");
        let session = store
            .sessions
            .get_mut(session_id)
            .ok_or_else(|| AppError::not_found(format!("未找到会话 `{session_id}`")))?;

        if session.ending_status == "ready" {
            return Err(AppError::bad_request("当前会话已经抵达结局，无法继续推进。"));
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
            .map_err(AppError::bad_request)?;
        apply_engine_snapshot(session, &snapshot);

        let mut archive_to_upsert = None;
        if snapshot.finished {
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
            archive_to_upsert = Some((session.session_id.clone(), ending));
        } else {
            session.history.push(HistoryItem {
                item_type: "narration".to_string(),
                turn_index: session.state_view.turn_index,
                text: session.current_node.text.clone(),
                created_at: now_string(),
            });
        }

        let response = SubmitChoiceData {
            accepted: true,
            session_id: session_id.to_string(),
            turn_id: session.state_view.active_turn_id,
            resource_delta,
            resources: session.resources.clone(),
            state_view: session.state_view.clone(),
        };

        if let Some((session_key, ending)) = archive_to_upsert {
            if let Some((archive_id, archive)) = store
                .sessions
                .get(&session_key)
                .map(|session| build_archive_record(session, &ending))
            {
                store.archives.insert(archive_id, archive);
            }
        }

        Ok(response)
    }

    pub fn get_game_session_ending(
        &self,
        session_id: &str,
    ) -> Result<GameSessionEndingData, AppError> {
        let store = self.store.lock().expect("mock store poisoned");
        let session = store
            .sessions
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

    pub fn create_intuition_preview(
        &self,
        session_id: &str,
        choice_id: &str,
    ) -> Result<IntuitionPreviewData, AppError> {
        let mut store = self.store.lock().expect("mock store poisoned");
        let session = store
            .sessions
            .get_mut(session_id)
            .ok_or_else(|| AppError::not_found(format!("未找到会话 `{session_id}`")))?;

        let choice = session
            .current_node
            .choices
            .iter()
            .find(|item| item.id == choice_id)
            .cloned()
            .ok_or_else(|| AppError::bad_request("无法为不存在的分支生成预览。"))?;

        if session.resources.intuition_points <= 0 {
            return Err(AppError::bad_request("直觉不足，无法窥探命运碎片。"));
        }

        session.resources.intuition_points -= 1;
        Ok(IntuitionPreviewData {
            choice_id: choice.id,
            preview_text: format!(
                "你窥见一段尚未凝固的未来：若踏上“{}”，{}会更快显露真正的震源，而你也会提前承受它的回响。",
                choice.text, session.world.core_conflict
            ),
            resource_delta: ResourceDelta {
                obsession_points: 0,
                intuition_points: -1,
            },
            resources: session.resources.clone(),
        })
    }

    pub fn get_game_session_history(&self, session_id: &str) -> Result<HistoryListData, AppError> {
        let store = self.store.lock().expect("mock store poisoned");
        let session = store
            .sessions
            .get(session_id)
            .ok_or_else(|| AppError::not_found(format!("未找到会话 `{session_id}`")))?;

        Ok(HistoryListData {
            items: session.history.clone(),
        })
    }

    pub fn stream_game_session(&self, session_id: &str) -> Result<StreamHandshakeData, AppError> {
        let store = self.store.lock().expect("mock store poisoned");
        if !store.sessions.contains_key(session_id) {
            return Err(AppError::not_found(format!("未找到会话 `{session_id}`")));
        }

        Ok(StreamHandshakeData {
            session_id: session_id.to_string(),
            protocol: "mock-polling".to_string(),
            note: "当前已接入 ECS 生成链路，但仍未启用真正的流式通道，可继续轮询会话快照与历史接口。"
                .to_string(),
        })
    }

    pub fn create_save(&self, _request: CreateSaveRequest) -> Result<SaveSummary, AppError> {
        Err(AppError::bad_request(
            "当前版本已优先打通核心故事链路，暂未恢复存档能力。",
        ))
    }

    pub fn list_saves(&self) -> SaveListData {
        let store = self.store.lock().expect("mock store poisoned");
        let mut items: Vec<SaveListItem> = store
            .saves
            .values()
            .map(|record| SaveListItem {
                save_id: record.summary.save_id.clone(),
                session_id: record.summary.session_id.clone(),
                title: record.summary.title.clone(),
                character_name: "".to_string(),
                background: "".to_string(),
                era: "".to_string(),
                turn_index: record.summary.turn_index,
                summary: record.summary.summary.clone(),
                cover_image: record.summary.cover_image.clone(),
                saved_at: record.summary.saved_at.clone(),
            })
            .collect();
        items.sort_by(|left, right| right.saved_at.cmp(&left.saved_at));
        SaveListData { items }
    }

    pub fn load_save(&self, _save_id: &str) -> Result<LoadSaveData, AppError> {
        Err(AppError::bad_request(
            "当前版本已优先打通核心故事链路，暂未恢复读档能力。",
        ))
    }

    pub fn list_archives(&self) -> ArchiveListData {
        let store = self.store.lock().expect("mock store poisoned");
        let mut items: Vec<ArchiveListItem> = store
            .archives
            .values()
            .map(|record| record.list_item.clone())
            .collect();
        items.sort_by(|left, right| right.created_at.cmp(&left.created_at));

        ArchiveListData { items }
    }

    pub fn get_archive(&self, archive_id: &str) -> Result<ArchiveDetailData, AppError> {
        let store = self.store.lock().expect("mock store poisoned");
        let archive = store
            .archives
            .get(archive_id)
            .ok_or_else(|| AppError::not_found(format!("未找到归档 `{archive_id}`")))?;

        Ok(archive.detail.clone())
    }

    pub fn generate_save_share_card(
        &self,
        save_id: &str,
        style: &str,
    ) -> Result<ShareCardData, AppError> {
        let store = self.store.lock().expect("mock store poisoned");
        if !store.saves.contains_key(save_id) {
            return Err(AppError::not_found(format!("未找到存档 `{save_id}`")));
        }

        Ok(build_share_card("save", save_id, style))
    }

    pub fn generate_ending_share_card(
        &self,
        archive_id: &str,
        style: &str,
    ) -> Result<ShareCardData, AppError> {
        let store = self.store.lock().expect("mock store poisoned");
        if !store.archives.contains_key(archive_id) {
            return Err(AppError::not_found(format!("未找到归档 `{archive_id}`")));
        }

        Ok(build_share_card("ending", archive_id, style))
    }
}

fn initial_resources(world: &World, snapshot: &AkashicSessionSnapshot) -> SessionResources {
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

fn apply_engine_snapshot(session: &mut SessionRecord, snapshot: &AkashicSessionSnapshot) {
    let display_turn_index = if snapshot.finished {
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
    session.status = if snapshot.finished {
        "completed".to_string()
    } else {
        "active".to_string()
    };
    session.ending_status = if snapshot.finished {
        "ready".to_string()
    } else {
        "pending".to_string()
    };
    session.current_node = StoryNode {
        id: format!("node-{}", display_turn_index),
        text: latest_history.clone(),
        image: cover_image_for(&session.world.era),
        choices: if snapshot.finished {
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
        game_state: if snapshot.finished {
            "ending".to_string()
        } else {
            "playing".to_string()
        },
        phase: if snapshot.finished {
            "EndingReady".to_string()
        } else {
            "AwaitingChoice".to_string()
        },
        turn_index: display_turn_index,
        active_turn_id: snapshot.active_turn_id,
        current_location: snapshot.world_snapshot.location_name.clone(),
        current_scene: if snapshot.finished {
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

fn seed_archives() -> Vec<ArchiveRecord> {
    vec![
        ArchiveRecord {
            list_item: ArchiveListItem {
                archive_id: "archive-seed-1".to_string(),
                title: "灰烬港的夜巡人".to_string(),
                tag: "已归档".to_string(),
                era: "末日废土".to_string(),
                summary: "你在塌陷的灯塔下守住最后的航标，让一座濒死聚落撑过风暴夜。"
                    .to_string(),
                cover_image: cover_image_for("末日废土"),
                created_at: now_string(),
            },
            detail: ArchiveDetailData {
                archive_id: "archive-seed-1".to_string(),
                title: "灰烬港的夜巡人".to_string(),
                era: "末日废土".to_string(),
                ending: EndingData {
                    biography: "在荒原风暴不断逼近的年代，你守住了最后一盏航标灯，让仍愿归乡的人不至于迷失在黑夜里。".to_string(),
                    turning_points: vec![
                        EndingTurningPoint {
                            cause: "灯塔能源濒临熄灭".to_string(),
                            effect: "你拆下自己的外骨骼供电模块续上最后的火种".to_string(),
                        },
                        EndingTurningPoint {
                            cause: "聚落内部出现背叛".to_string(),
                            effect: "你选择公开真相，而不是继续维持脆弱的秩序".to_string(),
                        },
                    ],
                    legacy: "你的坚持让幸存者记住：末日并不意味着放弃彼此。".to_string(),
                    cgs: vec![
                        cover_image_for("末日废土"),
                        cover_image_for("蒸汽朋克"),
                        cover_image_for("星际拓荒"),
                    ],
                },
            },
        },
        ArchiveRecord {
            list_item: ArchiveListItem {
                archive_id: "archive-seed-2".to_string(),
                title: "纸鹤坠入星潮".to_string(),
                tag: "已归档".to_string(),
                era: "星际拓荒".to_string(),
                summary: "你在外环殖民地拦截失控信号，把一封迟来的家书送回原主人手中。"
                    .to_string(),
                cover_image: cover_image_for("星际拓荒"),
                created_at: now_string(),
            },
            detail: ArchiveDetailData {
                archive_id: "archive-seed-2".to_string(),
                title: "纸鹤坠入星潮".to_string(),
                era: "星际拓荒".to_string(),
                ending: EndingData {
                    biography: "你沿着失控电波穿过边境星港，把漂流多年的一封家书送回收件人手中，也让沉默已久的殖民地重新愿意彼此开口。".to_string(),
                    turning_points: vec![
                        EndingTurningPoint {
                            cause: "导航阵列发生错位".to_string(),
                            effect: "你改写了跃迁窗口，换来了一次本不该存在的重逢".to_string(),
                        },
                        EndingTurningPoint {
                            cause: "家书里藏着殖民地旧案".to_string(),
                            effect: "你决定公开这段历史，让伤口终于能够愈合".to_string(),
                        },
                    ],
                    legacy: "你让冰冷星海里重新出现了人的温度。".to_string(),
                    cgs: vec![
                        cover_image_for("星际拓荒"),
                        cover_image_for("东方玄幻"),
                        cover_image_for("蒸汽朋克"),
                    ],
                },
            },
        },
    ]
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

fn build_archive_record(session: &SessionRecord, ending: &EndingData) -> (String, ArchiveRecord) {
    let archive_id = format!("archive-{}", session.session_id);
    let archive = ArchiveRecord {
        list_item: ArchiveListItem {
            archive_id: archive_id.clone(),
            title: format!("{}的回响录", session.character.name),
            tag: "已归档".to_string(),
            era: session.world.era.clone(),
            summary: ending.legacy.clone(),
            cover_image: ending
                .cgs
                .first()
                .cloned()
                .unwrap_or_else(|| cover_image_for(&session.world.era)),
            created_at: now_string(),
        },
        detail: ArchiveDetailData {
            archive_id: archive_id.clone(),
            title: format!("{}的回响录", session.character.name),
            era: session.world.era.clone(),
            ending: ending.clone(),
        },
    };
    (archive_id, archive)
}

fn build_share_card(kind: &str, subject_id: &str, style: &str) -> ShareCardData {
    ShareCardData {
        share_card_id: format!("share-{}", Uuid::new_v4().simple()),
        image_url: format!("https://cards.akashic.mock/{kind}/{subject_id}?style={style}"),
        expires_at: (Utc::now() + chrono::TimeDelta::hours(24)).to_rfc3339(),
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
