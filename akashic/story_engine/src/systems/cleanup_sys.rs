use bevy_ecs::{
    entity::Entity,
    system::{Commands, Query},
};

use crate::components::{
    agent::{FlowOwner, NarrationOutcome, SimulationOutcome},
    turn_flow::{TurnFlow, TurnStage},
};

// 在新一轮开始时，清理上一轮残留在实体上的结果组件。
pub fn cleanup_previous_turn_outcomes_system(
    mut commands: Commands,
    query_flows: Query<(Entity, &TurnFlow)>,
    query_agents: Query<
        (
            Entity,
            &FlowOwner,
            Option<&SimulationOutcome>,
            Option<&NarrationOutcome>,
        ),
    >,
) {
    for (flow_entity, turn_flow) in query_flows.iter() {
        if turn_flow.stage != TurnStage::TurnReady {
            continue;
        }

        for (agent_entity, owner, simulation_outcome, narration_outcome) in query_agents.iter() {
            if owner.0 != flow_entity {
                continue;
            }

            if let Some(outcome) = simulation_outcome
                && outcome.turn_id != turn_flow.active_turn_id
            {
                commands.entity(agent_entity).remove::<SimulationOutcome>();
            }

            if let Some(outcome) = narration_outcome
                && outcome.turn_id != turn_flow.active_turn_id
            {
                commands.entity(agent_entity).remove::<NarrationOutcome>();
            }
        }
    }
}
