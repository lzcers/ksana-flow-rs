use bevy_ecs::{
    entity::Entity,
    query::Without,
    system::{Commands, Query, ResMut},
};

use crate::{
    components::{
        agent::{
            AgentOutputKind, NarrationOutcome, OwnedAgentMut, PendingReasoning, ReadyAgentFilter,
            RunningReasoning, SessionOwner, SimulationOutcome,
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
    mut sessions: Query<(
        Entity,
        &mut TurnFlow,
        &ProtagonistDecisionState,
        &WorldSnapshot,
    )>,
    simulation_outcomes: Query<(&SessionOwner, &SimulationOutcome)>,
    mut narrators: Query<OwnedAgentMut, ReadyAgentFilter>,
) {
    for (session_entity, mut flow, decision_state, world_snapshot) in sessions.iter_mut() {
        if flow.stage != TurnStage::NarrationReady {
            continue;
        }
        let simulation_results = simulation_outcomes
            .iter()
            .filter(|(owner, _)| owner.0 == session_entity)
            .map(|(_, outcome)| outcome)
            .filter(|outcome| outcome.turn_id == flow.active_turn_id)
            .map(|outcome| outcome.content.as_str())
            .collect::<Vec<_>>()
            .join("\n\n");
        let prompt = format!(
            "{}\n\n【模拟阶段结果】\n{}",
            world_snapshot.to_story_prompt(Some(decision_state.committed_action())),
            simulation_results
        );

        let Some((narrator_entity, mut narrator, _)) =
            narrators.iter_mut().find(|(_, agent, owner)| {
                owner.0 == session_entity && agent.output_kind == AgentOutputKind::Narration
            })
        else {
            flow.stage = TurnStage::Failed;
            continue;
        };
        narrator.append_user_message(&prompt);
        commands.entity(narrator_entity).insert(PendingReasoning);
        flow.stage = TurnStage::NarrationRunning;
    }
}

pub fn narration_apply_system(
    mut commands: Commands,
    mut sessions: Query<(Entity, &mut TurnFlow, &WorldSnapshot)>,
    mut narrators: Query<OwnedAgentMut, Without<PendingReasoning>>,
    mut task_manager: ResMut<TaskManager>,
) {
    for (session_entity, mut flow, world_snapshot) in sessions.iter_mut() {
        if flow.stage != TurnStage::NarrationRunning {
            continue;
        }
        let Some((narrator_entity, mut narrator, _)) =
            narrators.iter_mut().find(|(_, agent, owner)| {
                owner.0 == session_entity && agent.output_kind == AgentOutputKind::Narration
            })
        else {
            flow.stage = TurnStage::Failed;
            continue;
        };
        let Some(result) = task_manager.task_result(narrator_entity).cloned() else {
            continue;
        };
        match result.status {
            TaskStatus::Done => {
                let Some(output) = task_manager
                    .take_result(narrator_entity)
                    .and_then(|result| result.output)
                else {
                    continue;
                };
                narrator.append_assistant_message(&output);
                commands
                    .entity(narrator_entity)
                    .remove::<RunningReasoning>()
                    .insert(NarrationOutcome {
                        turn_id: flow.active_turn_id,
                        content: output,
                    });
                if world_snapshot.is_ending {
                    flow.finish_story();
                } else {
                    flow.stage = TurnStage::ProtagonistReady;
                }
            }
            TaskStatus::Error => {
                task_manager.clear_task(narrator_entity);
                commands
                    .entity(narrator_entity)
                    .remove::<RunningReasoning>();
                flow.stage = TurnStage::Failed;
            }
            TaskStatus::Pending | TaskStatus::Running => {}
        }
    }
}
