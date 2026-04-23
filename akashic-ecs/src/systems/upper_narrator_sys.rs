use bevy_ecs::{
    entity::Entity,
    message::{MessageReader, MessageWriter},
    query::With,
    system::{Query, Res, ResMut},
};

use crate::{
    components::upper_narrator::UpperNarrator,
    resources::{
        task_manager::{TaskKind, TaskManager, TaskStatus},
        turn_state::{TurnPhase, TurnState},
    },
    turn_messages::{TurnCommand, TurnEvent},
};

pub fn upper_narrator_system(
    query: Query<(Entity, &UpperNarrator)>,
    mut command_reader: MessageReader<TurnCommand>,
    mut task_manager: ResMut<TaskManager>,
) {
    let Ok((entity, upper_narrator)) = query.single() else {
        return;
    };

    for command in command_reader.read() {
        if let TurnCommand::RequestNarration { .. } = command {
            if task_manager.task_status(entity).is_none() {
                task_manager.spawn_task(entity, TaskKind::Narration, upper_narrator.get_context());
            }
        }
    }
}

pub fn upper_narrator_result_apply_system(
    query: Query<Entity, With<UpperNarrator>>,
    turn_state: Res<TurnState>,
    mut task_manager: ResMut<TaskManager>,
    mut event_writer: MessageWriter<TurnEvent>,
) {
    if turn_state.phase != TurnPhase::AwaitingNarrationResult {
        return;
    }

    let Ok(entity) = query.single() else {
        return;
    };

    let Some(task_result) = task_manager.task_result(entity) else {
        return;
    };

    if task_result.kind != TaskKind::Narration {
        return;
    }

    match task_result.status {
        TaskStatus::Pending | TaskStatus::Running => {}
        TaskStatus::Done => {
            let summary = task_result
                .result
                .and_then(Result::ok)
                .unwrap_or_else(|| task_result.chunks.join(""));

            // TODO: 在这里把叙事文本落到事件流或展示缓冲区。
            event_writer.write(TurnEvent::NarrationCompleted {
                turn_id: turn_state.active_turn_id,
                summary,
            });
            task_manager.clear_task(entity);
        }
        TaskStatus::Error => {
            let message = task_result
                .result
                .and_then(Result::err)
                .unwrap_or_else(|| "narration task failed".to_string());

            event_writer.write(TurnEvent::TaskFailed {
                turn_id: turn_state.active_turn_id,
                stage: TurnPhase::AwaitingNarrationResult,
                entity,
                message,
            });
            task_manager.clear_task(entity);
        }
    }
}
