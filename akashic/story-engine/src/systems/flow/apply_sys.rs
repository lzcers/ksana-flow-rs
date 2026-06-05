use bevy_ecs::{entity::Entity, system::Query};

use crate::components::{
    agent::{Agent, ApplyOutcome, PipelinePhase, SessionOwner},
    turn_flow::{TurnFlow, TurnStage},
};

pub fn apply_progress_system(
    mut sessions: Query<(Entity, &mut TurnFlow)>,
    agents: Query<(&Agent, &SessionOwner, Option<&ApplyOutcome>)>,
) {
    for (session_entity, mut flow) in sessions.iter_mut().filter(|(_, flow)| {
        matches!(
            flow.stage,
            TurnStage::ApplicationReady | TurnStage::ApplicationRunning
        )
    }) {
        let mut apply_count = 0;
        let mut completed_count = 0;
        for (_, _, apply_outcome) in agents.iter().filter(|(agent, owner, ..)| {
            owner.0 == session_entity && agent.kind.pipeline_phase() == PipelinePhase::Application
        }) {
            apply_count += 1;
            completed_count += usize::from(
                apply_outcome.is_some_and(|outcome| outcome.turn_id == flow.active_turn_id),
            );
        }

        if apply_count == 0 {
            flow.stage = TurnStage::Failed;
            continue;
        }
        if completed_count < apply_count {
            continue;
        }
        flow.stage = TurnStage::AwaitingPlayerChoice;
    }
}
