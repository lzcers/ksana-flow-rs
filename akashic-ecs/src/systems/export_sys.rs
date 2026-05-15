use std::{
    collections::{HashMap, HashSet},
    io::{self, Write},
};

use bevy_ecs::{
    entity::Entity,
    system::{Local, Res, ResMut},
};

use crate::resources::{
    export::{ExportState, SessionSnapshot, TaskView},
    task_manager::{TaskManager, TaskStatus},
    turn_state::TurnState,
    world_snapshot::WorldSnapshot,
};

#[derive(Default)]
pub struct ChunkPrinterState {
    printed_chunk_offsets: HashMap<Entity, usize>,
    active_task: Option<Entity>,
}

pub fn export_system(
    turn_state: Res<TurnState>,
    world_snapshot: Res<WorldSnapshot>,
    export_state: Res<ExportState>,
    mut task_manager: ResMut<TaskManager>,
    mut printer_state: Local<ChunkPrinterState>,
) {
    // 获取任务快照，并转为 TaskView
    let task_results = task_manager.task_results_snapshot();
    let mut tasks = task_results
        .iter()
        .map(|(entity, result)| TaskView::from_task_result(format!("{entity:?}"), result.clone()))
        .collect::<Vec<_>>();
    tasks.sort_by(|left, right| left.entity.cmp(&right.entity));

    // 生成会话快照
    let snapshot = SessionSnapshot {
        phase: turn_state.phase,
        turn_index: turn_state.turn_index,
        active_turn_id: turn_state.active_turn_id,
        world: world_snapshot.clone(),
        tasks,
    };

    // 导出快照
    export_state.publish_snapshot(snapshot);

    // 导出任务更新
    for update in task_manager.drain_emitted_updates() {
        if let Some(task) = task_results
            .iter()
            .find(|(entity, _)| format!("{entity:?}") == update.entity)
            .map(|(_, result)| TaskView::from_task_result(update.entity.clone(), result.clone()))
        {
            export_state.publish_task_update(task, update);
        }
    }
    // 单独模块测试时打印任务 chunk
    // print_task_chunks(&task_manager, &mut printer_state);
}

// 打印任务的 task_chunk
fn print_task_chunks(task_manager: &TaskManager, printer_state: &mut ChunkPrinterState) {
    let task_results = task_manager.task_results_snapshot();
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
            print!("[task {:?} {:?}] ", entity, result.kind);
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
