use bevy_ecs::{
    entity::Entity,
    query::Without,
    system::{Commands, Query, Res, ResMut},
};
use serde_json::json;

use crate::{
    components::{
        agent::{Agent, AgentKind, FlowOwner, PendingReasoning, RunningReasoning},
        turn_flow::{TurnFlow, TurnStage},
    },
    resources::{
        llm_task_manager::{TaskManager, TaskStatus},
        protagonist_action::ProtagonistDecisionState,
        world_snapshot::WorldSnapshot,
    },
    utils::parse_json_response,
};

pub fn fate_dispatch_system(
    mut commands: Commands,
    mut query_flows: Query<(Entity, &mut TurnFlow)>,
    decision_state: Res<ProtagonistDecisionState>,
    mut query_agents: Query<
        (Entity, &mut Agent, &FlowOwner),
        (Without<PendingReasoning>, Without<RunningReasoning>),
    >,
) {
    for (flow_entity, mut flow) in query_flows.iter_mut() {
        if flow.stage != TurnStage::TurnReady {
            continue;
        }

        for (agent_entity, mut agent, owner) in query_agents.iter_mut() {
            if owner.0 != flow_entity || agent.kind != AgentKind::FateWeaver {
                continue;
            }

            let round = flow.active_turn_id.max(flow.turn_index + 1);
            let protagonist_action = decision_state.committed_action().to_string();
            agent.append_user_message(
                &json!({
                    "round": round,
                    "protagonist_action": protagonist_action
                })
                .to_string(),
            );
            commands.entity(agent_entity).insert(PendingReasoning);
            flow.stage = TurnStage::FateRunning;
        }
    }
}

pub fn fate_apply_system(
    mut commands: Commands,
    mut query_flows: Query<(Entity, &mut TurnFlow)>,
    mut query_agents: Query<(Entity, &mut Agent, &FlowOwner), Without<PendingReasoning>>,
    mut world_snapshot: ResMut<WorldSnapshot>,
    mut task_manager: ResMut<TaskManager>,
) {
    for (flow_entity, mut flow) in query_flows.iter_mut() {
        if flow.stage != TurnStage::FateRunning {
            continue;
        }

        for (agent_entity, mut agent, owner) in query_agents.iter_mut() {
            if owner.0 != flow_entity || agent.kind != AgentKind::FateWeaver {
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

                    match parse_json_response::<WorldSnapshot>(&raw_output) {
                        Ok(snapshot) => {
                            *world_snapshot = snapshot;
                            agent.append_assistant_message(&raw_output);
                            commands.entity(agent_entity).remove::<RunningReasoning>();
                            flow.stage = TurnStage::NarrationReady;
                        }
                        Err(message) => {
                            commands.entity(agent_entity).remove::<RunningReasoning>();
                            agent.revert();
                            flow.stage = if message.contains("无法解析 JSON 响应") {
                                TurnStage::TurnReady
                            } else {
                                TurnStage::Failed
                            };
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
