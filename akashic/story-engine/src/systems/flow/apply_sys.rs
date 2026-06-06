use bevy_ecs::{entity::Entity, query::With, system::Query};

use crate::components::{
    agent::{Applicator, SessionOwner},
    flow::ApplicationCompleted,
    turn_flow::{TurnFlow, TurnStage},
};

pub fn apply_progress_system(
    mut sessions: Query<(Entity, &mut TurnFlow)>,
    applicators: Query<&SessionOwner, With<Applicator>>,
    completed_applicators: Query<(&SessionOwner, &ApplicationCompleted), With<Applicator>>,
) {
    for (session_entity, mut flow) in sessions
        .iter_mut()
        .filter(|(_, flow)| flow.stage == TurnStage::Application)
    {
        let apply_count = applicators
            .iter()
            .filter(|owner| owner.0 == session_entity)
            .count();
        let completed_count = completed_applicators
            .iter()
            .filter(|(owner, completed)| {
                owner.0 == session_entity && completed.turn_id == flow.active_turn_id
            })
            .count();

        if apply_count == 0 {
            flow.stage = TurnStage::Failed;
            continue;
        }
        if completed_count < apply_count {
            continue;
        }
        flow.stage = TurnStage::AwaitingPlayer;
    }
}
