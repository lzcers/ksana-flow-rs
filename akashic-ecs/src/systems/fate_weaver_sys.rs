use bevy_ecs::{
    entity::Entity,
    system::{Query, Res, ResMut},
};

use crate::{
    components::fate_weaver::FateWeaver,
    resources::task_manager::{TaskKind, TaskManager},
    resources::turn_state::{TurnPhase, TurnState},
};

// FateWeaver 只负责在空闲阶段发起任务，不负责推进回合 phase。
pub fn fate_weaver_system(
    query: Query<(Entity, &FateWeaver)>,
    mut task_manager: ResMut<TaskManager>,
    turn_state: Res<TurnState>,
) {
    if turn_state.phase != TurnPhase::Idle {
        return;
    }

    let Ok((entity, fate_weaver)) = query.single() else {
        return;
    };

    if task_manager.task_status(entity).is_none() {
        task_manager.spawn_task(entity, TaskKind::FateWeaving, fate_weaver.get_context());
    }
}
