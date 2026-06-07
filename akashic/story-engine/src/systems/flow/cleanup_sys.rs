use bevy_ecs::{
    entity::Entity,
    system::{Commands, Query},
};

use crate::components::{
    agent::SessionOwner,
    flow::{ApplicationCompleted, SimulationCompleted},
    outcome::{NarrationOutcome, SimulationOutcome},
    turn_flow::{TurnFlow, TurnStage},
};

// 在新一轮开始时，清理上一轮残留在实体上的结果组件。
#[allow(clippy::type_complexity)]
pub fn cleanup_previous_turn_outcomes_system(
    mut commands: Commands,
    sessions: Query<(Entity, &TurnFlow)>,
    outcomes: Query<(
        Entity,
        &SessionOwner,
        Option<&SimulationOutcome>,
        Option<&NarrationOutcome>,
        Option<&SimulationCompleted>,
        Option<&ApplicationCompleted>,
    )>,
) {
    for (session_entity, turn_flow) in sessions
        .iter()
        .filter(|(_, flow)| flow.stage == TurnStage::Simulation)
    {
        for (agent_entity, _, simulation, narration, simulation_completed, application_completed) in
            outcomes
                .iter()
                .filter(|(_, owner, ..)| owner.0 == session_entity)
        {
            if simulation.is_some_and(|outcome| outcome.turn_id != turn_flow.active_turn_id) {
                commands.entity(agent_entity).remove::<SimulationOutcome>();
            }
            if narration.is_some_and(|outcome| outcome.turn_id != turn_flow.active_turn_id) {
                commands.entity(agent_entity).remove::<NarrationOutcome>();
            }
            if simulation_completed
                .is_some_and(|completed| completed.turn_id != turn_flow.active_turn_id)
            {
                commands
                    .entity(agent_entity)
                    .remove::<SimulationCompleted>();
            }
            if application_completed
                .is_some_and(|completed| completed.turn_id != turn_flow.active_turn_id)
            {
                commands
                    .entity(agent_entity)
                    .remove::<ApplicationCompleted>();
            }
        }
    }
}
