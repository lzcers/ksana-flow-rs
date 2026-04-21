use bevy_ecs::{
    entity::Entity,
    system::{Query, Res, ResMut},
};

use crate::{
    components::protagonist::Protagonist,
    resources::task_manager::{TaskKind, TaskManager},
    resources::turn_state::{TurnPhase, TurnState},
};

pub fn protagonist_system(
    query: Query<(Entity, &Protagonist)>,
    mut task_manager: ResMut<TaskManager>,
    turn_state: Res<TurnState>,
) {
    if turn_state.phase != TurnPhase::AwaitingProtagonist {
        return;
    }

    let Ok((entity, protagonist)) = query.single() else {
        return;
    };

    if task_manager.task_status(entity).is_none() {
        task_manager.spawn_task(
            entity,
            TaskKind::ProtagonistAction,
            protagonist.get_context(),
        );
    }
}
