use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
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

#[derive(Debug, Clone)]
pub struct AppState {
    pub service_name: &'static str,
    pub api_version: &'static str,
    store: Arc<Mutex<MockStore>>,
}

#[derive(Debug, Clone)]
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
}

#[derive(Debug, Clone)]
struct SaveRecord {
    summary: SaveSummary,
    snapshot: SessionRecord,
}

#[derive(Debug, Clone)]
struct ArchiveRecord {
    list_item: ArchiveListItem,
    detail: ArchiveDetailData,
}

#[derive(Debug)]
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
        let current_node = build_initial_node(&request.character, &request.world);
        let resources = SessionResources {
            obsession_points: 3,
            intuition_points: 5,
            days_left: 30,
            world_news: Some(format!(
                "{}的边缘地带传来异动，所有线索都在逼近“{}”的核心。",
                request.world.era, request.world.core_conflict
            )),
        };
        let state_view = build_state_view(
            "playing",
            "AwaitingChoice",
            1,
            &request.world.era,
            "命运开场",
            &current_node.text,
            resources.world_news.as_deref().unwrap_or(""),
            "尚未做出选择",
        );

        let session = SessionRecord {
            session_id: session_id.clone(),
            status: "active".to_string(),
            character: request.character.clone(),
            world: request.world.clone(),
            resources: resources.clone(),
            current_node: current_node.clone(),
            state_view: state_view.clone(),
            ending_status: "pending".to_string(),
            ending: None,
            history: vec![HistoryItem {
                item_type: "narration".to_string(),
                turn_index: 1,
                text: current_node.text.clone(),
                created_at: created_at.clone(),
            }],
        };

        let mut store = self.store.lock().expect("mock store poisoned");
        store.sessions.insert(session_id.clone(), session);

        Ok(CreateGameSessionData {
            session_id,
            created_at,
            character: request.character,
            world: request.world,
            resources,
            current_node,
            state_view,
        })
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
        let (response, archive_to_upsert) = {
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
            let turn_id = session.state_view.active_turn_id + 1;
            let choice_text = choice.text.clone();
            let mut resource_delta = ResourceDelta {
                obsession_points: 0,
                intuition_points: 0,
            };

            if request.use_obsession {
                session.resources.obsession_points -= 1;
                resource_delta.obsession_points = -1;
            }

            session.resources.days_left = (session.resources.days_left - 6).max(0);

            let world_news = build_world_news(previous_turn + 1, &session.world, &choice_text);
            session.resources.world_news = world_news.clone();
            session.history.push(HistoryItem {
                item_type: "choice".to_string(),
                turn_index: previous_turn,
                text: if request.use_obsession {
                    format!("{}（倾注执念）", choice_text)
                } else {
                    choice_text.clone()
                },
                created_at: now_string(),
            });

            let mut archive_to_upsert: Option<(SessionRecord, EndingData)> = None;

            if previous_turn >= 3 {
                let ending = build_ending(session, &choice_text, request.use_obsession);
                session.status = "completed".to_string();
                session.ending_status = "ready".to_string();
                session.ending = Some(ending.clone());
                session.state_view = build_state_view(
                    "ending",
                    "EndingReady",
                    previous_turn + 1,
                    &session.world.era,
                    "余响归档",
                    &ending.biography,
                    session.resources.world_news.as_deref().unwrap_or(""),
                    &choice_text,
                );
                session.history.push(HistoryItem {
                    item_type: "ending".to_string(),
                    turn_index: previous_turn + 1,
                    text: ending.legacy.clone(),
                    created_at: now_string(),
                });
                archive_to_upsert = Some((session.clone(), ending));
            } else {
                let next_turn = previous_turn + 1;
                let next_node =
                    build_story_node(session, next_turn, &choice_text, request.use_obsession);
                session.current_node = next_node.clone();
                session.state_view = build_state_view(
                    "playing",
                    "AwaitingChoice",
                    next_turn,
                    &session.world.era,
                    &format!("第{}幕", next_turn),
                    &next_node.text,
                    session.resources.world_news.as_deref().unwrap_or(""),
                    &choice_text,
                );
                session.history.push(HistoryItem {
                    item_type: "narration".to_string(),
                    turn_index: next_turn,
                    text: next_node.text,
                    created_at: now_string(),
                });
            }

            (
                SubmitChoiceData {
                    accepted: true,
                    session_id: session.session_id.clone(),
                    turn_id,
                    resource_delta,
                    resources: session.resources.clone(),
                    state_view: session.state_view.clone(),
                },
                archive_to_upsert,
            )
        };

        if let Some((session_snapshot, ending)) = archive_to_upsert {
            upsert_archive(&mut store.archives, &session_snapshot, &ending);
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
                "你窥见一段尚未凝固的未来：若踏上“{}”，{}的旧秘密会先一步暴露，而你会更早触碰到“{}”背后的代价。",
                choice.text, session.world.era, session.world.core_conflict
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
            note: "当前 mock 服务尚未启用真正的流式通道，可改用轮询会话快照与历史接口。"
                .to_string(),
        })
    }

    pub fn create_save(&self, request: CreateSaveRequest) -> Result<SaveSummary, AppError> {
        let mut store = self.store.lock().expect("mock store poisoned");
        let session = store
            .sessions
            .get(&request.session_id)
            .cloned()
            .ok_or_else(|| AppError::not_found(format!("未找到会话 `{}`", request.session_id)))?;

        let saved_at = now_string();
        let save_id = format!("save-{}", Uuid::new_v4().simple());
        let summary = SaveSummary {
            save_id: save_id.clone(),
            session_id: session.session_id.clone(),
            title: request.title,
            summary: build_save_summary(&session),
            cover_image: cover_image_for(&session.world.era),
            turn_index: session.state_view.turn_index,
            saved_at: saved_at.clone(),
        };

        store.saves.insert(
            save_id,
            SaveRecord {
                summary: summary.clone(),
                snapshot: session,
            },
        );

        Ok(summary)
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
                character_name: record.snapshot.character.name.clone(),
                background: record.snapshot.character.background.clone(),
                era: record.snapshot.world.era.clone(),
                turn_index: record.summary.turn_index,
                summary: record.summary.summary.clone(),
                cover_image: record.summary.cover_image.clone(),
                saved_at: record.summary.saved_at.clone(),
            })
            .collect();
        items.sort_by(|left, right| right.saved_at.cmp(&left.saved_at));

        SaveListData { items }
    }

    pub fn load_save(&self, save_id: &str) -> Result<LoadSaveData, AppError> {
        let mut store = self.store.lock().expect("mock store poisoned");
        let record = store
            .saves
            .get(save_id)
            .cloned()
            .ok_or_else(|| AppError::not_found(format!("未找到存档 `{save_id}`")))?;

        store
            .sessions
            .insert(record.snapshot.session_id.clone(), record.snapshot.clone());

        Ok(LoadSaveData {
            session_id: record.snapshot.session_id.clone(),
            loaded_from_save_id: save_id.to_string(),
            status: record.snapshot.status.clone(),
            resources: record.snapshot.resources.clone(),
            current_node: record.snapshot.current_node.clone(),
            state_view: record.snapshot.state_view.clone(),
        })
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

    pub fn generate_save_share_card(&self, save_id: &str, style: &str) -> Result<ShareCardData, AppError> {
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

fn build_initial_node(character: &Character, world: &World) -> StoryNode {
    StoryNode {
        id: "node-1".to_string(),
        text: format!(
            "雨水沿着屋檐滴落。你叫{}，是{}。在{}的夜幕下，“{}”像一层悄然逼近的雾，把整座城市都包进了它的阴影。怀里的信物仍在发烫，像是在催促你推开第一扇门。",
            character.name, character.background, world.era, world.core_conflict
        ),
        image: cover_image_for(&world.era),
        choices: build_choices(1),
    }
}

fn build_story_node(
    session: &SessionRecord,
    turn_index: u64,
    choice_text: &str,
    used_obsession: bool,
) -> StoryNode {
    let obsession_text = if used_obsession {
        "你把执念一起压进这次行动，命运因此给出了更激烈也更危险的回声。"
    } else {
        "你暂时压住心底那团火，只让理智和直觉带路。"
    };

    StoryNode {
        id: format!("node-{turn_index}"),
        text: format!(
            "你选择了“{}”。\n\n{}的空气随之发生偏折，新的线索开始浮现。{} 远处的秩序正出现裂纹，而你已经被推向更深的一层真相。",
            choice_text, session.world.era, obsession_text
        ),
        image: cover_image_for(&session.world.era),
        choices: build_choices(turn_index),
    }
}

fn build_choices(turn_index: u64) -> Vec<Choice> {
    match turn_index {
        1 => vec![
            choice("c1", "推门进入灯火深处，直接追问线索"),
            choice("c2", "沿着侧巷潜行，先摸清谁在布网"),
        ],
        2 => vec![
            choice("c3", "追踪那名突然失踪的目击者"),
            choice("c4", "与可疑盟友交易，换取一段禁忌情报"),
            choice("c5", "暂避锋芒，回到旧住处整理碎片"),
        ],
        _ => vec![
            choice("c6", "闯入风暴中心，亲手揭开真相"),
            choice("c7", "保护身边的人，哪怕放走幕后黑手"),
            choice("c8", "留下诱饵，逼迫命运先暴露底牌"),
        ],
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
            if used_obsession { "，并在最后一刻倾注了全部执念" } else { "" }
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

fn build_state_view(
    game_state: &str,
    phase: &str,
    turn_index: u64,
    era: &str,
    current_scene: &str,
    latest_history: &str,
    latest_broadcast_summary: &str,
    latest_protagonist_action: &str,
) -> RuntimeStateView {
    RuntimeStateView {
        game_state: game_state.to_string(),
        phase: phase.to_string(),
        turn_index,
        active_turn_id: turn_index,
        current_location: current_location_for(era).to_string(),
        current_scene: current_scene.to_string(),
        protagonist_state: "神经紧绷，但仍保有行动意志".to_string(),
        npcs_state: "周围角色正在根据你的选择重新站队".to_string(),
        latest_history: latest_history.to_string(),
        latest_broadcast_summary: latest_broadcast_summary.to_string(),
        latest_protagonist_action: latest_protagonist_action.to_string(),
    }
}

fn build_world_news(turn_index: u64, world: &World, choice_text: &str) -> Option<String> {
    match turn_index {
        2 => Some(format!(
            "{}传来急报：一支与你选择“{}”有关的势力正在重新调动人手。",
            world.era, choice_text
        )),
        3 => Some(format!(
            "关于“{}”的流言扩散到了整个{}，旧秩序开始显出裂纹。",
            world.core_conflict, world.era
        )),
        _ => None,
    }
}

fn build_save_summary(session: &SessionRecord) -> String {
    format!(
        "{}在{}已推进至第 {} 幕，当前焦点是“{}”。",
        session.character.name,
        session.world.era,
        session.state_view.turn_index,
        session.world.core_conflict
    )
}

fn upsert_archive(
    archives: &mut HashMap<String, ArchiveRecord>,
    session: &SessionRecord,
    ending: &EndingData,
) {
    let archive_id = format!("archive-{}", session.session_id);
    archives.insert(
        archive_id.clone(),
        ArchiveRecord {
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
                archive_id,
                title: format!("{}的回响录", session.character.name),
                era: session.world.era.clone(),
                ending: ending.clone(),
            },
        },
    );
}

fn build_share_card(kind: &str, subject_id: &str, style: &str) -> ShareCardData {
    ShareCardData {
        share_card_id: format!("share-{}", Uuid::new_v4().simple()),
        image_url: format!(
            "https://cards.akashic.mock/{kind}/{subject_id}?style={style}"
        ),
        expires_at: (Utc::now() + chrono::TimeDelta::hours(24)).to_rfc3339(),
    }
}

fn choice(id: &str, text: &str) -> Choice {
    Choice {
        id: id.to_string(),
        text: text.to_string(),
        disabled: false,
        cost_hints: ChoiceCostHints {
            intuition: 1,
            obsession: 1,
        },
    }
}

fn now_string() -> String {
    Utc::now().to_rfc3339()
}

fn current_location_for(era: &str) -> &'static str {
    match era {
        "蒸汽朋克" => "铸铁之城 · 下环区",
        "星际拓荒" => "边境星港 · 第七码头",
        "东方玄幻" => "雾隐城 · 长街",
        "末日废土" => "余烬聚落 · 风口哨站",
        _ => "灰雾城区 · 未知坐标",
    }
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
