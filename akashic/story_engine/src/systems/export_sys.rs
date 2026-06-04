use bevy_ecs::system::{Query, Res};

use crate::{
    components::{
        agent::{Agent, AgentKind, NarrationOutcome},
        turn_flow::TurnFlow,
    },
    resources::{
        export::{ExportState, SessionSnapshot, TaskView},
        history::SessionHistoryLog,
        llm_task_manager::{TaskManager, TaskStatus},
        protagonist_action::ProtagonistDecisionState,
        world_snapshot::WorldSnapshot,
    },
};

pub fn export_system(
    query_flow: Query<&TurnFlow>,
    query_agents: Query<(&Agent, Option<&NarrationOutcome>)>,
    world_snapshot: Res<WorldSnapshot>,
    history_log: Res<SessionHistoryLog>,
    decision_state: Res<ProtagonistDecisionState>,
    task_manager: Res<TaskManager>,
    export_state: Res<ExportState>,
) {
    let Ok(flow) = query_flow.single() else {
        return;
    };

    let latest_narration = query_agents
        .iter()
        .find_map(|(agent, outcome)| {
            if agent.kind != AgentKind::UpperNarrator {
                return None;
            }
            outcome
                .filter(|outcome| outcome.turn_id == flow.active_turn_id)
                .map(|outcome| outcome.content.clone())
        })
        .unwrap_or_default();

    let mut tasks: Vec<TaskView> = task_manager
        .results
        .iter()
        .map(|(entity, result)| TaskView::from_task_result(format!("{:?}", entity), result))
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
