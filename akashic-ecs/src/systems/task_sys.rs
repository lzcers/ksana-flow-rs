use std::{
    collections::{HashMap, HashSet},
    io::{self, Write},
};

use bevy_ecs::{
    entity::Entity,
    system::{Local, ResMut},
};

use crate::resources::task_manager::{TaskManager, TaskStatus};

#[derive(Default)]
pub struct ChunkPrinterState {
    printed_chunk_offsets: HashMap<Entity, usize>,
    active_task: Option<Entity>,
}

// 通用轮询层：只推进任务流，不解释业务含义。
pub fn task_system(
    mut task_manager: ResMut<TaskManager>,
    mut printer_state: Local<ChunkPrinterState>,
) {
    task_manager.poll_all_tasks();
    print_task_chunks(&task_manager, &mut printer_state);
}

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
