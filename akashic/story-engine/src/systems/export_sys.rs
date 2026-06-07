use bevy_ecs::{
    change_detection::{DetectChanges, Ref},
    entity::Entity,
    hierarchy::ChildOf,
    system::{Query, Res},
};

use crate::{
    components::{outcome::NarrationOutcome, turn_flow::TurnFlow},
    resources::{
        agent_task::{AgentTaskManager, TaskStatus},
        export::{ExportState, SessionSnapshot, TaskView},
        history::SessionHistoryLog,
        protagonist_action::ProtagonistDecisionState,
        world_snapshot::WorldSnapshot,
    },
};

#[allow(clippy::type_complexity)]
pub fn export_system(
    sessions: Query<(
        Entity,
        Ref<TurnFlow>,
        Ref<WorldSnapshot>,
        Ref<SessionHistoryLog>,
        Ref<ProtagonistDecisionState>,
        &ExportState,
    )>,
    agents: Query<(Entity, &ChildOf, Option<&NarrationOutcome>)>,
    agent_tasks: Res<AgentTaskManager>,
) {
    for (session_entity, flow, world_snapshot, history_log, decision_state, export_state) in
        sessions.iter().filter(
            |(_, flow, world_snapshot, history_log, decision_state, _)| {
                let is_active = !matches!(
                    flow.stage,
                    crate::components::turn_flow::TurnStage::Idle
                        | crate::components::turn_flow::TurnStage::AwaitingPlayer
                        | crate::components::turn_flow::TurnStage::TurnCompleted
                        | crate::components::turn_flow::TurnStage::Ended
                        | crate::components::turn_flow::TurnStage::Failed
                );
                is_active
                    || flow.is_changed()
                    || world_snapshot.is_changed()
                    || history_log.is_changed()
                    || decision_state.is_changed()
            },
        )
    {
        let latest_narration = agents
            .iter()
            .filter(|(_, owner, _)| owner.parent() == session_entity)
            .filter_map(|(_, _, narration)| narration)
            .find(|outcome| outcome.turn_id == flow.active_turn_id)
            .map(|outcome| outcome.content.clone())
            .unwrap_or_default();

        let mut tasks: Vec<TaskView> = agents
            .iter()
            .filter(|(_, owner, _)| owner.parent() == session_entity)
            .filter_map(|(entity, _, _)| {
                agent_tasks
                    .task_result(entity)
                    .map(|result| TaskView::from_task_result(format!("{entity:?}"), result))
            })
            .collect();
        tasks.sort_by(|a, b| a.entity.cmp(&b.entity));

        let current_task = tasks
            .iter()
            .find(|task| matches!(task.status, TaskStatus::Pending | TaskStatus::Running))
            .cloned();

        export_state.publish_snapshot(SessionSnapshot {
            phase: flow.stage,
            turn_index: flow.turn_index,
            active_turn_id: flow.active_turn_id,
            world: world_snapshot.clone(),
            history: history_log.rounds.clone(),
            current_task,
            tasks,
            latest_narration,
            current_protagonist_action: decision_state.committed_action().to_string(),
            choices: decision_state.choices().to_vec(),
        });
    }
}
