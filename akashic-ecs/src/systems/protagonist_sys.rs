use bevy_ecs::{
    entity::Entity,
    message::MessageWriter,
    query::With,
    system::{Query, Res, ResMut},
};

use crate::{
    components::protagonist::Protagonist,
    resources::{
        manual_control::ManualControlState,
        task_manager::{TaskKind, TaskManager, TaskStatus},
        turn_state::{TurnPhase, TurnState},
        world_state::WorldState,
    },
    turn_messages::TurnEvent,
    utils::{task_error_message, task_success_output, write_task_failed},
};

pub fn protagonist_system(
    turn_state: Res<TurnState>,
    manual_control: Res<ManualControlState>,
    world_state: Res<WorldState>,
    query: Query<(Entity, &Protagonist)>,
    mut task_manager: ResMut<TaskManager>,
) {
    let Ok((entity, protagonist)) = query.single() else {
        return;
    };

    if task_manager.task_status(entity).is_some() {
        return;
    }

    if manual_control.pending_protagonist_override_turn == Some(turn_state.active_turn_id) {
        return;
    }

    let Some(kind) = protagonist_task_kind(turn_state.phase) else {
        return;
    };

    let context = protagonist.build_task_context(&world_state);
    task_manager.spawn_task(entity, kind, &context);
}

pub fn protagonist_result_apply_system(
    query: Query<Entity, With<Protagonist>>,
    manual_control: Res<ManualControlState>,
    turn_state: Res<TurnState>,
    mut task_manager: ResMut<TaskManager>,
    mut event_writer: MessageWriter<TurnEvent>,
) {
    let Ok(entity) = query.single() else {
        return;
    };

    let Some(task_result) = task_manager.task_result(entity) else {
        return;
    };

    if task_result.kind != TaskKind::ProtagonistAction {
        return;
    }

    if manual_control.pending_protagonist_override_turn == Some(turn_state.active_turn_id) {
        task_manager.clear_task(entity);
        return;
    }

    // phase 已经离开主角决策阶段时，旧任务结果不再参与当前推进，直接清理避免阻塞下一次 spawn。
    let Some(expected_kind) = protagonist_task_kind(turn_state.phase) else {
        task_manager.clear_task(entity);
        return;
    };

    if task_result.kind != expected_kind {
        task_manager.clear_task(entity);
        return;
    }

    match task_result.status {
        TaskStatus::Pending | TaskStatus::Running => {}
        TaskStatus::Done => {
            let action_text = task_success_output(&task_result).trim().to_string();

            if action_text.is_empty() {
                write_task_failed(
                    &mut event_writer,
                    &turn_state,
                    entity,
                    "protagonist action is empty".to_string(),
                );
            } else {
                event_writer.write(TurnEvent::ProtagonistActionGenerated {
                    turn_id: turn_state.active_turn_id,
                    action_text,
                });
            }
            task_manager.clear_task(entity);
        }
        TaskStatus::Error => {
            let message = task_error_message(&task_result, "protagonist task failed");

            write_task_failed(&mut event_writer, &turn_state, entity, message);
            task_manager.clear_task(entity);
        }
    }
}

fn protagonist_task_kind(phase: TurnPhase) -> Option<TaskKind> {
    match phase {
        TurnPhase::AwaitingProtagonist => Some(TaskKind::ProtagonistAction),
        _ => None,
    }
}
