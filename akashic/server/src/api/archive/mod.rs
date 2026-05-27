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
