use std::{
    collections::{HashMap, HashSet},
    io::{self, Write},
};

use bevy_ecs::{
    entity::Entity,
    system::{Local, Res, ResMut},
};

use crate::components::upper_narrator::UpperNarrator;
use crate::resources::{
    export::{ExportState, SessionSnapshot, TaskView},
    protagonist_action::ProtagonistDecisionState,
    task_manager::{TaskManager, TaskStatus},
    turn_state::TurnState,
    world_snapshot::WorldSnapshot,
};

#[allow(dead_code)]
#[derive(Default)]
pub struct ChunkPrinterState {
    printed_chunk_offsets: HashMap<Entity, usize>,
    active_task: Option<Entity>,
}

pub fn export_system(
    turn_state: Res<TurnState>,
    world_snapshot: Res<WorldSnapshot>,
    decision_state: Res<ProtagonistDecisionState>,
    export_state: Res<ExportState>,
    mut task_manager: ResMut<TaskManager>,
    narrator_query: bevy_ecs::system::Query<&UpperNarrator>,
    mut _printer_state: Local<ChunkPrinterState>,
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
        .and_then(|narrator| narrator.context().print_latest_msg())
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

    // 导出任务更新
    for update in std::mem::take(&mut task_manager.emitted_updates) {
        if let Some(task) = task_results
            .iter()
            .find(|(entity, _)| format!("{entity:?}") == update.entity)
            .map(|(_, result)| TaskView::from_task_result(update.entity.clone(), result.clone()))
        {
            export_state.publish_task_update(task);
        }
    }
    // 单独模块测试时打印任务 chunk
    print_task_chunks(&task_manager, &mut _printer_state);
}

#[allow(dead_code)]
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

// 打印任务的 task_chunk
#[allow(dead_code)]
fn print_task_chunks(task_manager: &TaskManager, printer_state: &mut ChunkPrinterState) {
    let task_results = task_manager
        .results
        .iter()
        .map(|(entity, result)| (*entity, result.clone()))
        .collect::<Vec<_>>();
    let active_entities: HashSet<Entity> = task_results.iter().map(|(entity, _)| *entity).collect();
    let mut wrote_output = false;

    for (entity, result) in task_results {
        let offset = *printer_state
            .printed_chunk_offsets
            .entry(entity)
            .or_insert(0);
        let has_new_chunks = offset < result.chunks.len();

        if !has_new_chunks {
            if printer_state.active_task == Some(entity)
                && matches!(result.status, TaskStatus::Done | TaskStatus::Error)
            {
                println!();
                printer_state.active_task = None;
                wrote_output = true;
            }
            continue;
        }

        if printer_state.active_task != Some(entity) {
            if printer_state.active_task.is_some() {
                println!();
            }
            println!("[task {:?} {:?}] ", entity, result.kind);
            printer_state.active_task = Some(entity);
            wrote_output = true;
        }

        for chunk in result.chunks[offset..].iter() {
            print!("{chunk}");
            wrote_output = true;
        }

        printer_state
            .printed_chunk_offsets
            .insert(entity, result.chunks.len());

        if matches!(result.status, TaskStatus::Done | TaskStatus::Error) {
            println!();
            printer_state.active_task = None;
            wrote_output = true;
        }
    }

    if printer_state
        .active_task
        .is_some_and(|entity| !active_entities.contains(&entity))
    {
        println!();
        printer_state.active_task = None;
        wrote_output = true;
    }

    printer_state
        .printed_chunk_offsets
        .retain(|entity, _| active_entities.contains(entity));

    if wrote_output {
        let _ = io::stdout().flush();
    }
}
