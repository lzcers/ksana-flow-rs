mod dto;

pub use dto::*;

use akashic_ecs::engine::{AkashicSessionEngine, SessionArchiveState};

pub async fn gen_archive_payload(
    session_id: &str,
    title: Option<&str>,
    engine: &AkashicSessionEngine,
) -> Result<SessionArchivePayload, String> {
    let archive_state = engine.export_archive_state().await?;

    Ok(SessionArchivePayload {
        session_id: session_id.to_string(),
        title: archive_title(
            title,
            &archive_state.world_snapshot.scene_title,
            archive_state.active_turn_id,
        ),
        world_profile: archive_state.world_profile,
        protagonist_profile: archive_state.protagonist_profile,
        turn_state: TurnStateArchive {
            phase: archive_state.phase,
            turn_index: archive_state.turn_index,
            active_turn_id: archive_state.active_turn_id,
        },
        fate_weaver: archive_state.fate_weaver_context,
        upper_narrator: archive_state.upper_narrator_context,
        protagonist: archive_state.protagonist_context,
        world_snapshot: archive_state.world_snapshot,
        protagonist_decision: ProtagonistDecisionArchive {
            committed_action: archive_state.committed_action,
            choices: archive_state.choices,
        },
        history_log: archive_state.history_log,
    })
}

pub fn load_archive_payload(
    payload: SessionArchivePayload,
) -> Result<AkashicSessionEngine, String> {
    if payload.history_log.rounds.is_empty() {
        return Err("存档缺少 history_log，当前只接受包含完整时间线的新格式存档".to_string());
    }

    AkashicSessionEngine::from_archive_state(SessionArchiveState {
        world_profile: payload.world_profile,
        protagonist_profile: payload.protagonist_profile,
        phase: payload.turn_state.phase,
        turn_index: payload.turn_state.turn_index,
        active_turn_id: payload.turn_state.active_turn_id,
        world_snapshot: payload.world_snapshot,
        committed_action: payload.protagonist_decision.committed_action,
        choices: payload.protagonist_decision.choices,
        history_log: payload.history_log,
        fate_weaver_context: payload.fate_weaver,
        upper_narrator_context: payload.upper_narrator,
        protagonist_context: payload.protagonist,
    })
}

fn archive_title(input: Option<&str>, scene_title: &str, turn_index: u64) -> String {
    let provided = input.unwrap_or_default().trim();
    if !provided.is_empty() {
        return provided.to_string();
    }

    let scene = scene_title.trim();
    if !scene.is_empty() {
        return format!("第{}轮：{}", turn_index, scene);
    }

    format!("第{}轮存档", turn_index)
}
#[cfg(test)]
mod tests {
    use super::*;

    use agent::agent::context::Context;
    use akashic_ecs::resources::{
        history::SessionHistoryLog,
        protagonist_action::{PendingProtagonistChoice, ProtagonistOption},
        turn_state::TurnPhase,
        world_snapshot::WorldSnapshot,
    };
    use tokio::time::{Duration, sleep};

    #[tokio::test]
    async fn load_archive_payload_restores_engine_snapshot() {
        let payload = SessionArchivePayload {
            session_id: "session-load-test".to_string(),
            title: "第4轮：回廊阴影".to_string(),
            world_profile: "world".to_string(),
            protagonist_profile: "protagonist".to_string(),
            turn_state: TurnStateArchive {
                phase: TurnPhase::AwaitingPlayerChoice,
                turn_index: 4,
                active_turn_id: 4,
            },
            fate_weaver: Context::default(),
            upper_narrator: Context::default(),
            protagonist: Context::default(),
            world_snapshot: WorldSnapshot {
                round: 4,
                scene_title: "回廊阴影".to_string(),
                description: "湿冷的风从长廊深处卷来。".to_string(),
                ..WorldSnapshot::default()
            },
            protagonist_decision: ProtagonistDecisionArchive {
                committed_action: "贴墙潜行".to_string(),
                choices: vec![PendingProtagonistChoice {
                    id: "choice-1".to_string(),
                    option: ProtagonistOption {
                        title: "潜入".to_string(),
                        action: "贴墙潜行".to_string(),
                        motivation_and_risk: "能避开视线，但容易惊动暗处的人".to_string(),
                    },
                }],
            },
            history_log: SessionHistoryLog {
                rounds: vec![akashic_ecs::resources::history::RoundHistoryEntry {
                    round: 4,
                    world_snapshot: Some(WorldSnapshot {
                        round: 4,
                        scene_title: "回廊阴影".to_string(),
                        description: "湿冷的风从长廊深处卷来。".to_string(),
                        ..WorldSnapshot::default()
                    }),
                    narration_text: Some("长廊尽头传来回音。".to_string()),
                    choices: vec![PendingProtagonistChoice {
                        id: "choice-1".to_string(),
                        option: ProtagonistOption {
                            title: "潜入".to_string(),
                            action: "贴墙潜行".to_string(),
                            motivation_and_risk: "能避开视线，但容易惊动暗处的人".to_string(),
                        },
                    }],
                    committed_action: Some("贴墙潜行".to_string()),
                }],
            },
        };

        let engine = load_archive_payload(payload).expect("archive payload should load");
        sleep(Duration::from_millis(50)).await;

        let session = engine.get_game_session();
        assert_eq!(session.phase, TurnPhase::AwaitingPlayerChoice);
        assert_eq!(session.turn_index, 4);
        assert_eq!(session.active_turn_id, 4);
        assert_eq!(session.world_snapshot.scene_title, "回廊阴影");
        assert_eq!(session.current_protagonist_action, "贴墙潜行");
        assert_eq!(session.choices.len(), 1);
        assert_eq!(session.choices[0].option.action, "贴墙潜行");
    }

    #[test]
    fn load_archive_payload_rejects_empty_history_log() {
        let payload = SessionArchivePayload {
            session_id: "session-load-test".to_string(),
            title: "第4轮：回廊阴影".to_string(),
            world_profile: "world".to_string(),
            protagonist_profile: "protagonist".to_string(),
            turn_state: TurnStateArchive {
                phase: TurnPhase::AwaitingPlayerChoice,
                turn_index: 4,
                active_turn_id: 4,
            },
            fate_weaver: Context::default(),
            upper_narrator: Context::default(),
            protagonist: Context::default(),
            world_snapshot: WorldSnapshot {
                round: 4,
                scene_title: "回廊阴影".to_string(),
                ..WorldSnapshot::default()
            },
            protagonist_decision: ProtagonistDecisionArchive {
                committed_action: "贴墙潜行".to_string(),
                choices: vec![],
            },
            history_log: SessionHistoryLog::default(),
        };

        match load_archive_payload(payload) {
            Ok(_) => panic!("empty history log should be rejected"),
            Err(err) => assert!(err.contains("history_log")),
        }
    }

    #[test]
    fn session_archive_payload_requires_history_log_field() {
        let raw = r#"{
            "session_id":"session-load-test",
            "title":"第4轮：回廊阴影",
            "world_profile":"world",
            "protagonist_profile":"protagonist",
            "turn_state":{"phase":"awaiting_player_choice","turn_index":4,"active_turn_id":4},
            "fate_weaver":{"layers":[]},
            "upper_narrator":{"layers":[]},
            "protagonist":{"layers":[]},
            "world_snapshot":{"round":4,"scene_title":"回廊阴影","time_absolute":"","location_name":"","location_exits":[],"location_status":"","description":"","current_event":"","new_info":[],"inner_conflict":"","hard_anchors":[],"pace":"","atmosphere":"","focal_point":"","protagonist_condition":"","protagonist_known_secrets":[],"npcs":[],"items":[],"events_in_progress":[],"unsolved_threads":[],"pacing_note":""},
            "protagonist_decision":{"committed_action":"贴墙潜行","choices":[]}
        }"#;

        let err = serde_json::from_str::<SessionArchivePayload>(raw)
            .expect_err("payload without history_log should fail to deserialize");
        assert!(err.to_string().contains("history_log"));
    }
}
