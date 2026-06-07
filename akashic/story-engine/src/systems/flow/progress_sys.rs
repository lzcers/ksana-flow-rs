use bevy_ecs::{
    entity::Entity,
    hierarchy::ChildOf,
    query::With,
    system::{Commands, Query},
};

use crate::components::{
    agent::{Agent, AgentOutputType, Applicator, Simulator},
    flow::{ApplicationCompleted, FlowEnd, PlayerInputCompleted, SimulationCompleted},
    turn_flow::{TurnFlow, TurnStage},
};
use crate::resources::world_snapshot::WorldSnapshot;

#[allow(clippy::type_complexity)]
pub fn flow_progress_system(
    mut commands: Commands,
    mut sessions: Query<(
        Entity,
        &mut TurnFlow,
        &WorldSnapshot,
        Option<&FlowEnd>,
        Option<&PlayerInputCompleted>,
    )>,
    simulators: Query<&ChildOf, With<Simulator>>,
    completed_simulators: Query<(&ChildOf, &SimulationCompleted), With<Simulator>>,
    applicators: Query<(&ChildOf, &Agent), With<Applicator>>,
    completed_applicators: Query<(&ChildOf, &Agent, &ApplicationCompleted), With<Applicator>>,
) {
    for (session_entity, mut flow, world_snapshot, flow_end, player_input) in sessions.iter_mut() {
        match flow.stage {
            TurnStage::Simulation => {
                if flow_end.is_some() {
                    commands.entity(session_entity).remove::<FlowEnd>();
                    flow.end();
                    continue;
                }

                let total = simulators
                    .iter()
                    .filter(|owner| owner.parent() == session_entity)
                    .count();
                let completed = completed_simulators
                    .iter()
                    .filter(|(owner, completed)| {
                        owner.parent() == session_entity && completed.turn_id == flow.active_turn_id
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
                    .filter(|(owner, agent)| {
                        owner.parent() == session_entity
                            && (!world_snapshot.is_ending
                                || agent.output_type == AgentOutputType::Text)
                    })
                    .count();
                let completed = completed_applicators
                    .iter()
                    .filter(|(owner, agent, completed)| {
                        owner.parent() == session_entity
                            && completed.turn_id == flow.active_turn_id
                            && (!world_snapshot.is_ending
                                || agent.output_type == AgentOutputType::Text)
                    })
                    .count();

                if total == 0 {
                    flow.stage = TurnStage::Failed;
                } else if completed == total && world_snapshot.is_ending {
                    flow.end();
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

#[cfg(test)]
mod tests {
    use bevy_ecs::{prelude::*, schedule::Schedule};

    use super::*;

    #[test]
    fn ending_application_finishes_after_narration_without_player_input() {
        let mut world = World::new();
        let session = world
            .spawn((
                TurnFlow {
                    turn_index: 6,
                    active_turn_id: 7,
                    stage: TurnStage::Application,
                },
                WorldSnapshot {
                    is_ending: true,
                    ..WorldSnapshot::default()
                },
            ))
            .id();
        world.spawn((
            Agent::new(
                AgentOutputType::Text,
                "UpperNarrator",
                "write finale".to_string(),
            ),
            ChildOf(session),
            Applicator,
            ApplicationCompleted { turn_id: 7 },
        ));
        world.spawn((
            Agent::new(
                AgentOutputType::Json,
                "Protagonist",
                "choose action".to_string(),
            ),
            ChildOf(session),
            Applicator,
        ));
        let mut schedule = Schedule::default();
        schedule.add_systems(flow_progress_system);

        schedule.run(&mut world);

        let flow = world.get::<TurnFlow>(session).unwrap();
        assert_eq!(flow.stage, TurnStage::Ended);
        assert_eq!(flow.turn_index, 7);
        assert_eq!(flow.active_turn_id, 7);
    }
}
