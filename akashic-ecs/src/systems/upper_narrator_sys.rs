use bevy_ecs::{
    entity::Entity,
    system::{Query, Res, ResMut},
};

use crate::{
    components::upper_narrator::UpperNarrator,
    resources::task_manager::{TaskKind, TaskManager},
    resources::turn_state::{TurnPhase, TurnState},
};

pub fn upper_narrator_system(
    query: Query<(Entity, &UpperNarrator)>,
    mut task_manager: ResMut<TaskManager>,
    turn_state: Res<TurnState>,
) {
    if turn_state.phase != TurnPhase::AwaitingNarration {
        return;
    }

    let Ok((entity, upper_narrator)) = query.single() else {
        return;
    };

    if task_manager.task_status(entity).is_none() {
        task_manager.spawn_task(entity, TaskKind::Narration, upper_narrator.get_context());
    }
}
