use bevy_ecs::{
    entity::Entity,
    query::Without,
    system::{Commands, Query, ResMut},
};

use crate::{
    components::{
        agent::{
            AgentOutputKind, OwnedAgentMut, PendingReasoning, ReadyAgentFilter, RunningReasoning,
        },
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
    mut sessions: Query<(
        Entity,
        &mut TurnFlow,
        &ProtagonistDecisionState,
        &WorldSnapshot,
    )>,
    mut protagonists: Query<OwnedAgentMut, ReadyAgentFilter>,
) {
    for (session_entity, mut flow, decision_state, world_snapshot) in sessions.iter_mut() {
        if flow.stage != TurnStage::ProtagonistReady {
            continue;
        }
        let Some((protagonist_entity, mut protagonist, _)) =
            protagonists.iter_mut().find(|(_, agent, owner)| {
                owner.0 == session_entity
                    && agent.output_kind == AgentOutputKind::ProtagonistOptions
            })
        else {
            flow.stage = TurnStage::Failed;
            continue;
        };
        protagonist.append_user_message(
            &world_snapshot.to_protagonist_prompt(Some(decision_state.committed_action())),
        );
        commands.entity(protagonist_entity).insert(PendingReasoning);
        flow.stage = TurnStage::ProtagonistRunning;
    }
}

pub fn protagonist_apply_system(
    mut commands: Commands,
    mut sessions: Query<(Entity, &mut TurnFlow, &mut ProtagonistDecisionState)>,
    mut protagonists: Query<OwnedAgentMut, Without<PendingReasoning>>,
    mut task_manager: ResMut<TaskManager>,
) {
    for (session_entity, mut flow, mut decision_state) in sessions.iter_mut() {
        if flow.stage != TurnStage::ProtagonistRunning {
            continue;
        }
        let Some((protagonist_entity, mut protagonist, _)) =
            protagonists.iter_mut().find(|(_, agent, owner)| {
                owner.0 == session_entity
                    && agent.output_kind == AgentOutputKind::ProtagonistOptions
            })
        else {
            flow.stage = TurnStage::Failed;
            continue;
        };
        let Some(result) = task_manager.task_result(protagonist_entity).cloned() else {
            continue;
        };
        match result.status {
            TaskStatus::Done => {
                let Some(output) = task_manager
                    .take_result(protagonist_entity)
                    .and_then(|result| result.output)
                else {
                    continue;
                };
                let Ok(options) = parse_json_response::<ProtagonistOptions>(&output) else {
                    protagonist.revert();
                    commands
                        .entity(protagonist_entity)
                        .remove::<RunningReasoning>();
                    flow.stage = TurnStage::Failed;
                    continue;
                };
                if options.is_empty() {
                    commands
                        .entity(protagonist_entity)
                        .remove::<RunningReasoning>();
                    flow.stage = TurnStage::Failed;
                    continue;
                }
                protagonist.append_assistant_message(&output);
                commands
                    .entity(protagonist_entity)
                    .remove::<RunningReasoning>();
                decision_state.replace_with_options(options);
                flow.stage = TurnStage::AwaitingPlayerChoice;
            }
            TaskStatus::Error => {
                task_manager.clear_task(protagonist_entity);
                commands
                    .entity(protagonist_entity)
                    .remove::<RunningReasoning>();
                flow.stage = TurnStage::Failed;
            }
            TaskStatus::Pending | TaskStatus::Running => {}
        }
    }
}
