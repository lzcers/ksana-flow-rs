use bevy_ecs::{
    entity::Entity,
    query::With,
    system::{Commands, Query},
};

use crate::components::{
    agent::{SessionOwner, Simulator},
    flow::{FlowEnd, SimulationCompleted},
    turn_flow::{TurnFlow, TurnStage},
};

pub fn simulator_progress_system(
    mut commands: Commands,
    mut sessions: Query<(Entity, &mut TurnFlow)>,
    simulators: Query<&SessionOwner, With<Simulator>>,
    completed_simulators: Query<(&SessionOwner, &SimulationCompleted), With<Simulator>>,
    ended_flows: Query<(), With<FlowEnd>>,
) {
    for (session_entity, mut flow) in sessions
        .iter_mut()
        .filter(|(_, flow)| flow.stage == TurnStage::Simulation)
    {
        if ended_flows.contains(session_entity) {
            commands.entity(session_entity).remove::<FlowEnd>();
            flow.end();
            continue;
        }

        let simulator_count = simulators
            .iter()
            .filter(|owner| owner.0 == session_entity)
            .count();
        let completed_count = completed_simulators
            .iter()
            .filter(|(owner, completed)| {
                owner.0 == session_entity && completed.turn_id == flow.active_turn_id
            })
            .count();

        if simulator_count == 0 {
            flow.stage = TurnStage::Failed;
        } else if completed_count == simulator_count {
            flow.stage = TurnStage::Application;
        }
    }
}
