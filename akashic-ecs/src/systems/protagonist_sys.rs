use bevy_ecs::{
    entity::Entity,
    message::{MessageReader, MessageWriter},
    system::{Query, Res, ResMut},
};

use crate::{
    components::protagonist::Protagonist,
    resources::{
        task_manager::{TaskKind, TaskManager, TaskStatus},
        turn_state::{TurnPhase, TurnState},
    },
    turn_messages::{TurnCommand, TurnEvent},
};

pub fn protagonist_system(
    query: Query<(Entity, &Protagonist)>,
    mut command_reader: MessageReader<TurnCommand>,
    mut task_manager: ResMut<TaskManager>,
    turn_state: Res<TurnState>,
    mut event_writer: MessageWriter<TurnEvent>,
) {
    let Ok((entity, protagonist)) = query.single() else {
        return;
    };

    for command in command_reader.read() {
        if let TurnCommand::RequestProtagonist { turn_id } = command {
            if *turn_id != turn_state.active_turn_id {
                continue;
            }

            if task_manager.task_status(entity).is_none() {
                task_manager.spawn_task(
                    entity,
                    TaskKind::ProtagonistAction,
                    protagonist.get_context(),
                );
            }
        }
    }

    if turn_state.phase != TurnPhase::AwaitingProtagonistResult {
        return;
    }

    let Some(snapshot) = task_manager.task_result(entity) else {
        return;
    };

    if snapshot.kind != TaskKind::ProtagonistAction {
        return;
    }

    match snapshot.status {
        TaskStatus::Pending | TaskStatus::Running => {}
        TaskStatus::Done => {
            let summary = snapshot
                .result
                .and_then(Result::ok)
                .unwrap_or_else(|| snapshot.chunks.join(""));

            // TODO: 在这里把主角行动结果回写到共享上下文。
            event_writer.write(TurnEvent::ProtagonistDecided {
                turn_id: turn_state.active_turn_id,
                summary,
            });
            task_manager.clear_task(entity);
        }
        TaskStatus::Error => {
            let message = snapshot
                .result
                .and_then(Result::err)
                .unwrap_or_else(|| "protagonist task failed".to_string());

            event_writer.write(TurnEvent::TaskFailed {
                turn_id: turn_state.active_turn_id,
                stage: TurnPhase::AwaitingProtagonistResult,
                entity,
                message,
            });
            task_manager.clear_task(entity);
        }
    }
}
