use bevy_ecs::{
    entity::Entity,
    hierarchy::ChildOf,
    query::{With, Without},
    system::{Commands, Query, Res, ResMut},
};
use serde_json::json;

use crate::{
    components::{
        agent::{Agent, AgentOutputType, PendingReasoning, Simulator},
        flow::{FlowEnd, SimulationCompleted},
        outcome::SimulationOutcome,
        turn_flow::{TurnFlow, TurnStage},
    },
    resources::{
        agent_task::{AgentTaskManager, TaskStatus},
        protagonist_action::ProtagonistDecisionState,
        world_snapshot::WorldSnapshot,
    },
    utils::parse_json_response,
};

#[allow(clippy::type_complexity)]
pub fn fate_weaver_dispatch_system(
    mut commands: Commands,
    sessions: Query<(Entity, &TurnFlow, &ProtagonistDecisionState, &WorldSnapshot)>,
    agent_tasks: Res<AgentTaskManager>,
    mut agents: Query<
        (Entity, &mut Agent, &ChildOf),
        (
            With<Simulator>,
            Without<PendingReasoning>,
            Without<SimulationOutcome>,
        ),
    >,
) {
    for (session_entity, flow, decision_state, world_snapshot) in sessions
        .iter()
        .filter(|(_, flow, ..)| flow.stage == TurnStage::Simulation)
    {
        if world_snapshot.is_ending {
            commands.entity(session_entity).insert(FlowEnd);
            continue;
        }

        for (entity, mut agent, _) in agents.iter_mut().filter(|(entity, _, owner)| {
            owner.parent() == session_entity && agent_tasks.task_result(*entity).is_none()
        }) {
            agent.append_user_message(
                &json!({
                    "round": flow.active_turn_id.max(flow.turn_index + 1),
                    "protagonist_action": decision_state.committed_action(),
                    "previous_world_snapshot": world_snapshot,
                })
                .to_string(),
            );
            commands.entity(entity).insert(PendingReasoning);
        }
    }
}

#[allow(clippy::type_complexity)]
pub fn fate_weaver_apply_system(
    mut commands: Commands,
    mut sessions: Query<(Entity, &mut TurnFlow, &mut WorldSnapshot)>,
    mut agents: Query<(Entity, &mut Agent, &ChildOf), With<Simulator>>,
    mut agent_tasks: ResMut<AgentTaskManager>,
) {
    for (session_entity, mut flow, mut world_snapshot) in sessions
        .iter_mut()
        .filter(|(_, flow, _)| flow.stage == TurnStage::Simulation)
    {
        for (entity, mut agent, _) in agents
            .iter_mut()
            .filter(|(_, _, owner)| owner.parent() == session_entity)
        {
            let Some(result) = agent_tasks.task_result(entity).cloned() else {
                continue;
            };
            match result.status {
                TaskStatus::Done => {
                    let Some(output) = agent_tasks
                        .take_result(entity)
                        .and_then(|result| result.output)
                    else {
                        continue;
                    };

                    if agent.output_type == AgentOutputType::Json {
                        let Ok(snapshot) = parse_json_response::<WorldSnapshot>(&output) else {
                            agent.revert();
                            flow.stage = TurnStage::Failed;
                            break;
                        };
                        *world_snapshot = snapshot;
                    }
                    agent.append_assistant_message(&output);
                    commands
                        .entity(entity)
                        .insert(SimulationOutcome {
                            turn_id: flow.active_turn_id,
                            content: output,
                        })
                        .insert(SimulationCompleted {
                            turn_id: flow.active_turn_id,
                        });
                }
                TaskStatus::Error => {
                    flow.stage = TurnStage::Failed;
                    break;
                }
                TaskStatus::Pending | TaskStatus::Running => {}
            }
        }
    }
}
