use bevy_ecs::{
    entity::Entity,
    query::Without,
    system::{Commands, Query, ResMut},
};

use crate::{
    components::{
        agent::{
            AgentOutputType, OwnedAgentMut, PendingReasoning, PipelinePhase, ReadyAgentFilter,
            RunningReasoning,
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
    sessions: Query<(Entity, &TurnFlow, &ProtagonistDecisionState, &WorldSnapshot)>,
    mut agents: Query<OwnedAgentMut, ReadyAgentFilter>,
) {
    for (session_entity, _flow, decision_state, world_snapshot) in sessions
        .iter()
        .filter(|(_, flow, ..)| flow.stage == TurnStage::ApplicationReady)
    {
        if world_snapshot.is_ending {
            continue;
        }
        let prompt = world_snapshot.to_protagonist_prompt(Some(decision_state.committed_action()));

        for (entity, mut agent, _) in agents.iter_mut().filter(|(_, agent, owner)| {
            owner.0 == session_entity
                && agent.kind.pipeline_phase() == PipelinePhase::Application
                && agent.output_type == AgentOutputType::Json
        }) {
            agent.append_user_message(&prompt);
            commands.entity(entity).insert(PendingReasoning);
        }
    }
}

pub fn protagonist_apply_system(
    mut commands: Commands,
    mut sessions: Query<(Entity, &mut TurnFlow, &mut ProtagonistDecisionState)>,
    mut agents: Query<OwnedAgentMut, Without<PendingReasoning>>,
    mut task_manager: ResMut<TaskManager>,
) {
    for (session_entity, mut flow, mut decision_state) in sessions
        .iter_mut()
        .filter(|(_, flow, _)| flow.stage == TurnStage::ApplicationRunning)
    {
        for (entity, mut agent, _) in agents.iter_mut().filter(|(_, agent, owner)| {
            owner.0 == session_entity
                && agent.kind.pipeline_phase() == PipelinePhase::Application
                && agent.output_type == AgentOutputType::Json
        }) {
            let Some(result) = task_manager.task_result(entity).cloned() else {
                continue;
            };
            match result.status {
                TaskStatus::Done => {
                    let Some(output) = task_manager
                        .take_result(entity)
                        .and_then(|result| result.output)
                    else {
                        continue;
                    };
                    let Ok(options) = parse_json_response::<ProtagonistOptions>(&output) else {
                        agent.revert();
                        commands.entity(entity).remove::<RunningReasoning>();
                        flow.stage = TurnStage::Failed;
                        break;
                    };
                    if options.is_empty() {
                        commands.entity(entity).remove::<RunningReasoning>();
                        flow.stage = TurnStage::Failed;
                        break;
                    }
                    agent.append_assistant_message(&output);
                    decision_state.replace_with_options(options);
                    commands.entity(entity).remove::<RunningReasoning>();
                }
                TaskStatus::Error => {
                    task_manager.clear_task(entity);
                    commands.entity(entity).remove::<RunningReasoning>();
                    flow.stage = TurnStage::Failed;
                    break;
                }
                TaskStatus::Pending | TaskStatus::Running => {}
            }
        }
    }
}
