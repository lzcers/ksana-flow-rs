use bevy_ecs::{
    change_detection::{DetectChanges, Mut, Ref},
    entity::Entity,
    hierarchy::ChildOf,
    system::Query,
};

use crate::{
    components::{
        outcome::NarrationOutcome,
        turn_flow::{TurnFlow, TurnStage},
    },
    resources::{
        history::SessionHistoryLog, protagonist_action::ProtagonistDecisionState,
        world_snapshot::WorldSnapshot,
    },
};

#[allow(clippy::type_complexity)]
pub fn history_sys(
    mut sessions: Query<(
        Entity,
        Ref<TurnFlow>,
        Ref<WorldSnapshot>,
        Ref<ProtagonistDecisionState>,
        Mut<SessionHistoryLog>,
    )>,
    narrations: Query<(&ChildOf, Ref<NarrationOutcome>)>,
) {
    for (session_entity, flow, world_snapshot, decision_state, mut history_log) in sessions
        .iter_mut()
        .filter(|(_, flow, ..)| flow.active_turn_id != 0)
    {
        let narration_changed = narrations
            .iter()
            .filter(|(owner, _)| owner.parent() == session_entity)
            .any(|(_, outcome)| outcome.is_changed());
        if !flow.is_changed()
            && !world_snapshot.is_changed()
            && !decision_state.is_changed()
            && !narration_changed
        {
            continue;
        }

        history_log.set_world_snapshot(flow.active_turn_id, world_snapshot.clone());

        if let Some((_, outcome)) = narrations.iter().find(|(owner, outcome)| {
            owner.parent() == session_entity && outcome.turn_id == flow.active_turn_id
        }) {
            history_log.set_narration(flow.active_turn_id, outcome.content.clone());
        }

        if !decision_state.choices().is_empty() {
            history_log.set_choices(flow.active_turn_id, decision_state.choices().to_vec());
        }

        if matches!(flow.stage, TurnStage::TurnCompleted | TurnStage::Ended) {
            history_log.set_committed_action(
                flow.active_turn_id,
                decision_state.committed_action().to_string(),
            );
        }
    }
}
