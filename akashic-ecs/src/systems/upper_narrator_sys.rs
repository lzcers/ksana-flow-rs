use bevy_ecs::{
    entity::Entity,
    system::{Query, ResMut},
};

use crate::{
    components::upper_narrator::UpperNarrator,
    resources::task_manager::{TaskKind, TaskManager, TaskStatus},
    resources::turn_state::{TurnPhase, TurnState},
};

pub fn upper_narrator_system(
    query: Query<(Entity, &UpperNarrator)>,
    mut task_manager: ResMut<TaskManager>,
    mut turn_state: ResMut<TurnState>,
) {
    if turn_state.phase != TurnPhase::AwaitingNarration {
        return;
    }

    let Ok((entity, upper_narrator)) = query.single() else {
        return;
    };

    match task_manager.task_status(entity) {
        None => {
            task_manager.spawn_task(entity, TaskKind::Narration, upper_narrator.get_context());
        }
        Some(TaskStatus::Pending | TaskStatus::Running) => {}
        Some(TaskStatus::Done) => {
            // TODO: 在这里读取叙事任务结果，并把结果回写到共享上下文或事件流。
            task_manager.clear_task(entity);
            turn_state.phase = TurnPhase::Idle;
        }
        Some(TaskStatus::Error) => {
            // TODO: 补充叙事任务失败态与恢复策略。
            task_manager.clear_task(entity);
            turn_state.phase = TurnPhase::Idle;
        }
    }
}
