use std::io::{self, Write};

use agent::core::Message;
use bevy_ecs::system::{Res, ResMut};

use crate::components::upper_narrator::UpperNarrator;
use crate::resources::{
    export::{ExportState, SessionSnapshot, TaskView},
    protagonist_action::ProtagonistDecisionState,
    task_manager::{TaskManager, TaskStatus, TaskUpdate},
    turn_state::TurnState,
    world_snapshot::WorldSnapshot,
};

pub fn export_system(
    turn_state: Res<TurnState>,
    world_snapshot: Res<WorldSnapshot>,
    decision_state: Res<ProtagonistDecisionState>,
    export_state: Res<ExportState>,
    mut task_manager: ResMut<TaskManager>,
    narrator_query: bevy_ecs::system::Query<&UpperNarrator>,
) {
    // 获取任务快照，并转为 TaskView
    let task_results = task_manager
        .results
        .iter()
        .map(|(entity, result)| (*entity, result.clone()))
        .collect::<Vec<_>>();
    let mut tasks = task_results
        .iter()
        .map(|(entity, result)| TaskView::from_task_result(format!("{entity:?}"), result.clone()))
        .collect::<Vec<_>>();
    tasks.sort_by(|left, right| left.entity.cmp(&right.entity));
    let current_task = select_current_task(&tasks);
    let latest_narration = narrator_query
        .single()
        .ok()
        .and_then(|narrator| {
            latest_assistant_narration(narrator.context().conversation().as_slice())
        })
        .unwrap_or_default();
    let current_protagonist_action = decision_state.committed_action().to_string();
    let choices = decision_state.choices().to_vec();

    // 生成会话快照
    let snapshot = SessionSnapshot {
        phase: turn_state.phase,
        turn_index: turn_state.turn_index,
        active_turn_id: turn_state.active_turn_id,
        world: world_snapshot.clone(),
        current_task,
        tasks,
        latest_narration,
        current_protagonist_action,
        choices,
    };

    // 导出快照
    export_state.publish_snapshot(snapshot);

    // 导出任务更新。这里只发送增量更新，完整快照由 snapshot 单独承担。
    let updates = std::mem::take(&mut task_manager.emitted_updates);
    for update in &updates {
        export_state.publish_task_update(update.clone());
    }
    // 单独模块测试时打印任务 chunk
    print_task_updates(&updates);
}

fn select_current_task(tasks: &[TaskView]) -> Option<TaskView> {
    tasks
        .iter()
        .find(|task| matches!(task.status, TaskStatus::Running))
        .or_else(|| {
            tasks
                .iter()
                .find(|task| matches!(task.status, TaskStatus::Pending))
        })
        .cloned()
}

fn latest_assistant_narration(messages: &[Message]) -> Option<String> {
    messages.iter().rev().find_map(|message| match message {
        Message::Assistant { content, .. } if !content.trim().is_empty() => {
            Some(content.trim().to_string())
        }
        _ => None,
    })
}

// 打印任务的 task_chunk 增量
fn print_task_updates(updates: &[TaskUpdate]) {
    let mut wrote_output = false;

    for update in updates {
        if let Some(chunk) = &update.chunk {
            print!("{chunk}");
            wrote_output = true;
        }

        if matches!(update.status, TaskStatus::Done | TaskStatus::Error) {
            println!();
            wrote_output = true;
        }
    }

    if wrote_output {
        let _ = io::stdout().flush();
    }
}
