use bevy_ecs::{
    entity::Entity,
    query::{With, Without},
    system::{Commands, Query, ResMut},
};
use serde_json::json;

use crate::{
    components::{
        agent::{
            Agent, AgentOutputType, PendingReasoning, RunningReasoning, SessionOwner, Simulator,
        },
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
    mut agents: Query<
        (Entity, &mut Agent, &SessionOwner),
        (
            With<Simulator>,
            Without<PendingReasoning>,
            Without<RunningReasoning>,
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

        for (entity, mut agent, _) in agents
            .iter_mut()
            .filter(|(_, _, owner)| owner.0 == session_entity)
        {
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
    mut agents: Query<
        (Entity, &mut Agent, &SessionOwner),
        (With<Simulator>, Without<PendingReasoning>),
    >,
    mut agent_tasks: ResMut<AgentTaskManager>,
) {
    for (session_entity, mut flow, mut world_snapshot) in sessions
        .iter_mut()
        .filter(|(_, flow, _)| flow.stage == TurnStage::Simulation)
    {
        for (entity, mut agent, _) in agents
            .iter_mut()
            .filter(|(_, _, owner)| owner.0 == session_entity)
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
                            commands.entity(entity).remove::<RunningReasoning>();
                            flow.stage = TurnStage::Failed;
                            break;
                        };
                        *world_snapshot = snapshot;
                    }
                    agent.append_assistant_message(&output);
                    commands
                        .entity(entity)
                        .remove::<RunningReasoning>()
                        .insert(SimulationOutcome {
                            turn_id: flow.active_turn_id,
                            content: output,
                        })
                        .insert(SimulationCompleted {
                            turn_id: flow.active_turn_id,
                        });
                }
                TaskStatus::Error => {
                    agent_tasks.clear_task(entity);
                    commands.entity(entity).remove::<RunningReasoning>();
                    flow.stage = TurnStage::Failed;
                    break;
                }
                TaskStatus::Pending | TaskStatus::Running => {}
            }
        }
    }
}
