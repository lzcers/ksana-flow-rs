use bevy_ecs::{
    entity::Entity,
    query::Without,
    system::{Commands, Query, ResMut},
};

use crate::{
    components::{
        agent::{
            AgentOutputType, NarrationOutcome, OwnedAgentMut, PendingReasoning, PipelinePhase,
            ReadyAgentFilter, RunningReasoning, SessionOwner, SimulationOutcome,
        },
        turn_flow::{TurnFlow, TurnStage},
    },
    resources::{
        llm_task_manager::{TaskManager, TaskStatus},
        protagonist_action::ProtagonistDecisionState,
        world_snapshot::WorldSnapshot,
    },
};

pub fn narration_dispatch_system(
    mut commands: Commands,
    sessions: Query<(Entity, &TurnFlow, &ProtagonistDecisionState, &WorldSnapshot)>,
    simulation_outcomes: Query<(&SessionOwner, &SimulationOutcome)>,
    mut agents: Query<OwnedAgentMut, ReadyAgentFilter>,
) {
    for (session_entity, flow, decision_state, world_snapshot) in sessions
        .iter()
        .filter(|(_, flow, ..)| flow.stage == TurnStage::ApplicationReady)
    {
        let simulation_results = simulation_outcomes
            .iter()
            .filter(|(owner, outcome)| {
                owner.0 == session_entity && outcome.turn_id == flow.active_turn_id
            })
            .map(|(_, outcome)| outcome.content.as_str())
            .collect::<Vec<_>>()
            .join("\n\n");
        let prompt = format!(
            "{}\n\n【模拟阶段结果】\n{}",
            world_snapshot.to_story_prompt(Some(decision_state.committed_action())),
            simulation_results
        );

        for (entity, mut agent, _) in agents.iter_mut().filter(|(_, agent, owner)| {
            owner.0 == session_entity
                && agent.kind.pipeline_phase() == PipelinePhase::Application
                && agent.output_type == AgentOutputType::Text
        }) {
            agent.append_user_message(&prompt);
            commands.entity(entity).insert(PendingReasoning);
        }
    }
}

pub fn narration_apply_system(
    mut commands: Commands,
    mut sessions: Query<(Entity, &mut TurnFlow)>,
    mut agents: Query<OwnedAgentMut, Without<PendingReasoning>>,
    mut task_manager: ResMut<TaskManager>,
) {
    for (session_entity, mut flow) in sessions
        .iter_mut()
        .filter(|(_, flow)| flow.stage == TurnStage::ApplicationRunning)
    {
        for (entity, mut agent, _) in agents.iter_mut().filter(|(_, agent, owner)| {
            owner.0 == session_entity
                && agent.kind.pipeline_phase() == PipelinePhase::Application
                && agent.output_type == AgentOutputType::Text
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
                    agent.append_assistant_message(&output);
                    commands
                        .entity(entity)
                        .remove::<RunningReasoning>()
                        .insert(NarrationOutcome {
                            turn_id: flow.active_turn_id,
                            content: output,
                        });
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
