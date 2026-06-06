mod dto;

pub use dto::*;

use std::io::Write;

use akashic_ecs::engine::{AkashicEngine, AkashicSessionEngine, SessionArchiveState};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use flate2::{Compression, read::GzDecoder, write::GzEncoder};

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
        key_story_beats: archive_state.key_story_beats,
        turn_state: TurnStateArchive {
            phase: archive_state.phase,
            turn_index: archive_state.turn_index,
            active_turn_id: archive_state.active_turn_id,
        },
        fate_weaver: archive_state.fate_weaver_context,
        upper_narrator: archive_state.upper_narrator_context,
        protagonist: archive_state.protagonist_context,
        simulators: archive_state.simulators,
        world_snapshot: archive_state.world_snapshot,
        protagonist_decision: ProtagonistDecisionArchive {
            committed_action: archive_state.committed_action,
            choices: archive_state.choices,
        },
        history_log: archive_state.history_log,
    })
}

pub async fn load_archive_payload(
    engine: &AkashicEngine,
    payload: SessionArchivePayload,
) -> Result<AkashicSessionEngine, String> {
    if payload.history_log.rounds.is_empty() {
        return Err("存档缺少 history_log，当前只接受包含完整时间线的新格式存档".to_string());
    }

    let session_id = payload.session_id.clone();
    engine
        .create_session_from_archive(
            session_id,
            SessionArchiveState {
                world_profile: payload.world_profile,
                protagonist_profile: payload.protagonist_profile,
                key_story_beats: payload.key_story_beats,
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
                simulators: payload.simulators,
            },
        )
        .await
}

pub fn validate_archive_payload(payload: &SessionArchivePayload) -> Result<(), String> {
    if payload.session_id.trim().is_empty() {
        return Err("存档缺少 `session_id`。".to_string());
    }
    if payload.title.trim().is_empty() {
        return Err("存档缺少 `title`。".to_string());
    }
    if payload.history_log.rounds.is_empty() {
        return Err("存档缺少 history_log，当前只接受包含完整时间线的新格式存档".to_string());
    }

    Ok(())
}

pub fn compress_archive_payload(payload: &SessionArchivePayload) -> Result<String, String> {
    let archive_json =
        serde_json::to_vec(payload).map_err(|err| format!("序列化存档失败：{err}"))?;
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder
        .write_all(&archive_json)
        .map_err(|err| format!("压缩存档失败：{err}"))?;
    let compressed = encoder
        .finish()
        .map_err(|err| format!("完成存档压缩失败：{err}"))?;

    Ok(STANDARD.encode(compressed))
}

pub fn decompress_archive_payload(
    compressed_archive: &str,
) -> Result<SessionArchivePayload, String> {
    let compressed = STANDARD
        .decode(compressed_archive)
        .map_err(|err| format!("解码压缩存档失败：{err}"))?;
    let mut decoder = GzDecoder::new(compressed.as_slice());
    serde_json::from_reader(&mut decoder).map_err(|err| format!("解析压缩存档失败：{err}"))
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
        history::{RoundHistoryEntry, SessionHistoryLog},
        protagonist_action::{PendingProtagonistChoice, ProtagonistOption},
        turn_state::TurnPhase,
        world_snapshot::WorldSnapshot,
    };

    #[test]
    fn compress_archive_payload_returns_base64_gzip_text() {
        let payload = SessionArchivePayload {
            session_id: "session-compress".to_string(),
            title: "第3轮：塔楼回响".to_string(),
            world_profile: "world".to_string(),
            protagonist_profile: "protagonist".to_string(),
            key_story_beats: "beats".to_string(),
            turn_state: TurnStateArchive {
                phase: TurnPhase::AwaitingPlayer,
                turn_index: 3,
                active_turn_id: 3,
            },
            fate_weaver: Context::default(),
            upper_narrator: Context::default(),
            protagonist: Context::default(),
            simulators: vec![],
            world_snapshot: WorldSnapshot {
                round: 3,
                scene_title: "塔楼回响".to_string(),
                ..WorldSnapshot::default()
            },
            protagonist_decision: ProtagonistDecisionArchive {
                committed_action: "推门".to_string(),
                choices: vec![PendingProtagonistChoice {
                    id: "choice-1".to_string(),
                    option: ProtagonistOption {
                        title: "推门".to_string(),
                        action: "推门".to_string(),
                        motivation_and_risk: "可能惊动楼上的守望者".to_string(),
                    },
                }],
            },
            history_log: SessionHistoryLog {
                rounds: vec![RoundHistoryEntry {
                    round: 3,
                    world_snapshot: Some(WorldSnapshot {
                        round: 3,
                        scene_title: "塔楼回响".to_string(),
                        ..WorldSnapshot::default()
                    }),
                    narration_text: Some("旧钟仍在震颤。".to_string()),
                    choices: vec![],
                    committed_action: Some("推门".to_string()),
                }],
            },
        };

        let compressed = compress_archive_payload(&payload).expect("compression should succeed");

        assert!(!compressed.is_empty());
        assert!(compressed.len() > 10);
    }

    #[test]
    fn compress_and_decompress_archive_payload_round_trip() {
        let payload = SessionArchivePayload {
            session_id: "session-round-trip".to_string(),
            title: "第4轮：潮声之门".to_string(),
            world_profile: "world".to_string(),
            protagonist_profile: "protagonist".to_string(),
            key_story_beats: "beats".to_string(),
            turn_state: TurnStateArchive {
                phase: TurnPhase::AwaitingPlayer,
                turn_index: 4,
                active_turn_id: 4,
            },
            fate_weaver: Context::default(),
            upper_narrator: Context::default(),
            protagonist: Context::default(),
            simulators: vec![],
            world_snapshot: WorldSnapshot {
                round: 4,
                scene_title: "潮声之门".to_string(),
                ..WorldSnapshot::default()
            },
            protagonist_decision: ProtagonistDecisionArchive {
                committed_action: "开门".to_string(),
                choices: vec![],
            },
            history_log: SessionHistoryLog {
                rounds: vec![RoundHistoryEntry {
                    round: 4,
                    world_snapshot: Some(WorldSnapshot {
                        round: 4,
                        scene_title: "潮声之门".to_string(),
                        ..WorldSnapshot::default()
                    }),
                    narration_text: Some("海风从门缝里灌进来。".to_string()),
                    choices: vec![],
                    committed_action: Some("开门".to_string()),
                }],
            },
        };

        let compressed = compress_archive_payload(&payload).expect("compression should succeed");
        let restored =
            decompress_archive_payload(&compressed).expect("decompression should succeed");

        assert_eq!(restored.session_id, payload.session_id);
        assert_eq!(restored.title, payload.title);
        assert_eq!(
            restored.turn_state.turn_index,
            payload.turn_state.turn_index
        );
    }
}
