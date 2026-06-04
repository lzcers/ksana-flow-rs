use bevy_ecs::{
    entity::Entity,
    system::{Commands, Query},
};

use crate::components::{
    agent::{NarrationOutcome, SessionOwner, SimulationOutcome},
    turn_flow::{TurnFlow, TurnStage},
};

// 在新一轮开始时，清理上一轮残留在实体上的结果组件。
pub fn cleanup_previous_turn_outcomes_system(
    mut commands: Commands,
    sessions: Query<(Entity, &TurnFlow)>,
    outcomes: Query<(
        Entity,
        &SessionOwner,
        Option<&SimulationOutcome>,
        Option<&NarrationOutcome>,
    )>,
) {
    for (session_entity, turn_flow) in sessions.iter() {
        if turn_flow.stage != TurnStage::SimulationReady {
            continue;
        }

        for (agent_entity, owner, simulation, narration) in outcomes.iter() {
            if owner.0 != session_entity {
                continue;
            }
            if simulation.is_some_and(|outcome| outcome.turn_id != turn_flow.active_turn_id) {
                commands.entity(agent_entity).remove::<SimulationOutcome>();
            }
            if narration.is_some_and(|outcome| outcome.turn_id != turn_flow.active_turn_id) {
                commands.entity(agent_entity).remove::<NarrationOutcome>();
            }
        }
    }
}
