use bevy_ecs::{
    entity::Entity,
    query::Without,
    system::{Commands, Query, Res, ResMut},
};

use crate::{
    components::{
        agent::{Agent, AgentKind, FlowOwner, PendingReasoning, RunningReasoning},
        turn_flow::{TurnFlow, TurnStage},
    },
    resources::{
        llm_task_manager::{TaskManager, TaskStatus},
        protagonist_action::{ProtagonistDecisionState, ProtagonistOptions},
        world_snapshot::WorldSnapshot,
    },
    utils::parse_json_response,
};

pub fn protagonist_dispatch_system(
    mut commands: Commands,
    mut query_flows: Query<(Entity, &mut TurnFlow)>,
    decision_state: Res<ProtagonistDecisionState>,
    world_snapshot: Res<WorldSnapshot>,
    mut query_agents: Query<
        (Entity, &mut Agent, &FlowOwner),
        (Without<PendingReasoning>, Without<RunningReasoning>),
    >,
) {
    for (flow_entity, mut flow) in query_flows.iter_mut() {
        if flow.stage != TurnStage::ProtagonistReady {
            continue;
        }

        for (agent_entity, mut agent, owner) in query_agents.iter_mut() {
            if owner.0 != flow_entity || agent.kind != AgentKind::Protagonist {
                continue;
            }

            let protagonist_action = decision_state.committed_action();
            agent.append_user_message(&world_snapshot.to_protagonist_prompt(Some(protagonist_action)));
            commands.entity(agent_entity).insert(PendingReasoning);
            flow.stage = TurnStage::ProtagonistRunning;
        }
    }
}

pub fn protagonist_apply_system(
    mut commands: Commands,
    mut query_flows: Query<(Entity, &mut TurnFlow)>,
    mut query_agents: Query<(Entity, &mut Agent, &FlowOwner), Without<PendingReasoning>>,
    mut decision_state: ResMut<ProtagonistDecisionState>,
    mut task_manager: ResMut<TaskManager>,
) {
    for (flow_entity, mut flow) in query_flows.iter_mut() {
        if flow.stage != TurnStage::ProtagonistRunning {
            continue;
        }

        for (agent_entity, mut agent, owner) in query_agents.iter_mut() {
            if owner.0 != flow_entity || agent.kind != AgentKind::Protagonist {
                continue;
            }

            let Some(task_result) = task_manager.task_result(agent_entity).cloned() else {
                continue;
            };

            match task_result.status {
                TaskStatus::Done => {
                    let Some(raw_output) =
                        task_manager.take_result(agent_entity).and_then(|result| result.output)
                    else {
                        continue;
                    };

                    match parse_json_response::<ProtagonistOptions>(&raw_output) {
                        Ok(action_list) if !action_list.is_empty() => {
                            agent.append_assistant_message(&raw_output);
                            decision_state.replace_with_options(action_list);
                            commands.entity(agent_entity).remove::<RunningReasoning>();
                            flow.stage = TurnStage::AwaitingPlayerChoice;
                        }
                        Ok(_) => {
                            commands.entity(agent_entity).remove::<RunningReasoning>();
                            flow.stage = TurnStage::Failed;
                        }
                        Err(_) => {
                            agent.revert();
                            commands.entity(agent_entity).remove::<RunningReasoning>();
                            flow.stage = TurnStage::ProtagonistReady;
                        }
                    }
                }
                TaskStatus::Error => {
                    task_manager.clear_task(agent_entity);
                    commands.entity(agent_entity).remove::<RunningReasoning>();
                    flow.stage = TurnStage::Failed;
                }
                TaskStatus::Pending | TaskStatus::Running => {}
            }
        }
    }
}
