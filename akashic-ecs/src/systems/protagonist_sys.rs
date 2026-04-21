use bevy_ecs::{
    entity::Entity,
    system::{Query, ResMut},
};

use crate::{
    components::protagonist::Protagonist,
    resources::task_manager::{TaskKind, TaskManager, TaskStatus},
    resources::turn_state::{TurnPhase, TurnState},
};

pub fn protagonist_system(
    query: Query<(Entity, &Protagonist)>,
    mut task_manager: ResMut<TaskManager>,
    mut turn_state: ResMut<TurnState>,
) {
    if turn_state.phase != TurnPhase::AwaitingProtagonist {
        return;
    }

    let Ok((entity, protagonist)) = query.single() else {
        return;
    };

    match task_manager.task_status(entity) {
        None => {
            task_manager.spawn_task(
                entity,
                TaskKind::ProtagonistAction,
                protagonist.get_context(),
            );
        }
        Some(TaskStatus::Pending | TaskStatus::Running) => {}
        Some(TaskStatus::Done) => {
            // TODO: 在这里读取主角任务结果，并把结果回写到共享上下文。
            task_manager.clear_task(entity);
            turn_state.phase = TurnPhase::AwaitingNarration;
        }
        Some(TaskStatus::Error) => {
            // TODO: 补充主角任务失败态与恢复策略。
            task_manager.clear_task(entity);
            turn_state.phase = TurnPhase::Idle;
        }
    }
}
