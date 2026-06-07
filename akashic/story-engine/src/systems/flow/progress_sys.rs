use bevy_ecs::{
    entity::Entity,
    query::With,
    system::{Commands, Query},
};

use crate::components::{
    agent::{Applicator, SessionOwner, Simulator},
    flow::{ApplicationCompleted, FlowEnd, PlayerInputCompleted, SimulationCompleted},
    turn_flow::{TurnFlow, TurnStage},
};

#[allow(clippy::type_complexity)]
pub fn flow_progress_system(
    mut commands: Commands,
    mut sessions: Query<(
        Entity,
        &mut TurnFlow,
        Option<&FlowEnd>,
        Option<&PlayerInputCompleted>,
    )>,
    simulators: Query<&SessionOwner, With<Simulator>>,
    completed_simulators: Query<(&SessionOwner, &SimulationCompleted), With<Simulator>>,
    applicators: Query<&SessionOwner, With<Applicator>>,
    completed_applicators: Query<(&SessionOwner, &ApplicationCompleted), With<Applicator>>,
) {
    for (session_entity, mut flow, flow_end, player_input) in sessions.iter_mut() {
        match flow.stage {
            TurnStage::Simulation => {
                if flow_end.is_some() {
                    commands.entity(session_entity).remove::<FlowEnd>();
                    flow.end();
                    continue;
                }

                let total = simulators
                    .iter()
                    .filter(|owner| owner.0 == session_entity)
                    .count();
                let completed = completed_simulators
                    .iter()
                    .filter(|(owner, completed)| {
                        owner.0 == session_entity && completed.turn_id == flow.active_turn_id
                    })
                    .count();

                if total == 0 {
                    flow.stage = TurnStage::Failed;
                } else if completed == total {
                    flow.stage = TurnStage::Application;
                }
            }
            TurnStage::Application => {
                let total = applicators
                    .iter()
                    .filter(|owner| owner.0 == session_entity)
                    .count();
                let completed = completed_applicators
                    .iter()
                    .filter(|(owner, completed)| {
                        owner.0 == session_entity && completed.turn_id == flow.active_turn_id
                    })
                    .count();

                if total == 0 {
                    flow.stage = TurnStage::Failed;
                } else if completed == total {
                    flow.stage = TurnStage::AwaitingPlayer;
                }
            }
            TurnStage::AwaitingPlayer => {
                if player_input.is_some_and(|completed| completed.turn_id == flow.active_turn_id) {
                    commands
                        .entity(session_entity)
                        .remove::<PlayerInputCompleted>();
                    flow.finish_turn();
                }
            }
            TurnStage::Idle | TurnStage::TurnCompleted | TurnStage::Ended | TurnStage::Failed => {}
        }
    }
}
