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
            archive_state.turn_index,
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

pub fn load_archive_payload(
    payload: SessionArchivePayload,
) -> Result<AkashicSessionEngine, String> {
    AkashicSessionEngine::from_archive_state(SessionArchiveState {
        world_profile: payload.world_profile,
        protagonist_profile: payload.protagonist_profile,
        phase: payload.turn_state.phase,
        turn_index: payload.turn_state.turn_index,
        active_turn_id: payload.turn_state.active_turn_id,
        world_snapshot: payload.world_snapshot,
        committed_action: payload.protagonist_decision.committed_action,
        choices: payload.protagonist_decision.choices,
        fate_weaver_context: payload.fate_weaver,
        upper_narrator_context: payload.upper_narrator,
        protagonist_context: payload.protagonist,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    use agent::agent::context::Context;
    use akashic_ecs::resources::{
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
}
