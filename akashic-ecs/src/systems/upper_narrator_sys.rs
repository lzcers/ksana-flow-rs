use bevy_ecs::{
    entity::Entity,
    message::{MessageReader, MessageWriter},
    system::{Query, Res, ResMut},
};

use crate::{
    components::upper_narrator::UpperNarrator,
    resources::{
        task_manager::{TaskKind, TaskManager, TaskStatus},
        turn_state::{TurnPhase, TurnState},
    },
    turn_messages::{TurnCommand, TurnOutcome},
};

pub fn upper_narrator_system(
    query: Query<(Entity, &UpperNarrator)>,
    mut command_reader: MessageReader<TurnCommand>,
    mut task_manager: ResMut<TaskManager>,
    turn_state: Res<TurnState>,
    mut outcome_writer: MessageWriter<TurnOutcome>,
) {
    let Ok((entity, upper_narrator)) = query.single() else {
        return;
    };

    for command in command_reader.read() {
        if let TurnCommand::RequestNarration { turn_id } = command {
            if *turn_id != turn_state.active_turn_id {
                continue;
            }

            if task_manager.task_status(entity).is_none() {
                task_manager.spawn_task(entity, TaskKind::Narration, upper_narrator.get_context());
            }
        }
    }

    if turn_state.phase != TurnPhase::AwaitingNarrationResult {
        return;
    }

    let Some(snapshot) = task_manager.task_result(entity) else {
        return;
    };

    if snapshot.kind != TaskKind::Narration {
        return;
    }

    match snapshot.status {
        TaskStatus::Pending | TaskStatus::Running => {}
        TaskStatus::Done => {
            let summary = snapshot
                .result
                .and_then(Result::ok)
                .unwrap_or_else(|| snapshot.chunks.join(""));

            // TODO: 在这里把叙事文本落到事件流或展示缓冲区。
            outcome_writer.write(TurnOutcome::NarrationApplied {
                turn_id: turn_state.active_turn_id,
                summary,
            });
            task_manager.clear_task(entity);
        }
        TaskStatus::Error => {
            let message = snapshot
                .result
                .and_then(Result::err)
                .unwrap_or_else(|| "narration task failed".to_string());

            outcome_writer.write(TurnOutcome::TaskFailed {
                turn_id: turn_state.active_turn_id,
                stage: TurnPhase::AwaitingNarrationResult,
                entity,
                message,
            });
            task_manager.clear_task(entity);
        }
    }
}
