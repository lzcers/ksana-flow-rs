use bevy_ecs::system::{Query, Res, ResMut};

use crate::{
    components::{
        agent::{Agent, AgentKind, NarrationOutcome},
        turn_flow::{TurnFlow, TurnStage},
    },
    resources::{
        history::SessionHistoryLog,
        protagonist_action::ProtagonistDecisionState,
        world_snapshot::WorldSnapshot,
    },
};

pub fn history_sys(
    query_flow: Query<&TurnFlow>,
    query_agents: Query<(&Agent, Option<&NarrationOutcome>)>,
    world_snapshot: Res<WorldSnapshot>,
    decision_state: Res<ProtagonistDecisionState>,
    mut history_log: ResMut<SessionHistoryLog>,
) {
    let Ok(flow) = query_flow.single() else {
        return;
    };
    if flow.active_turn_id == 0 {
        return;
    }

    history_log.set_world_snapshot(flow.active_turn_id, world_snapshot.clone());

    for (agent, outcome) in query_agents.iter() {
        if agent.kind == AgentKind::UpperNarrator
            && let Some(outcome) = outcome
            && outcome.turn_id == flow.active_turn_id
        {
            history_log.set_narration(flow.active_turn_id, outcome.content.clone());
        }
    }

    if !decision_state.choices().is_empty() {
        history_log.set_choices(flow.active_turn_id, decision_state.choices().to_vec());
    }

    if matches!(flow.stage, TurnStage::TurnComplete | TurnStage::StoryEnded) {
        history_log.set_committed_action(
            flow.active_turn_id,
            decision_state.committed_action().to_string(),
        );
    }
}
