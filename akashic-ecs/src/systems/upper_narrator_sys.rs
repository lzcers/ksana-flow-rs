use bevy_ecs::{
    entity::Entity,
    system::{Query, ResMut},
};

use crate::{
    components::{
        AkashicState,
        fate_weaver::{FateLine, FateWeaver},
        protagonist::Protagonist,
        upper_narrator::UpperNarrator,
    },
    resources::task_manager::{TaskManager, TaskStatus},
};

pub fn upper_narrator_system(
    mut query: Query<(Entity, &UpperNarrator, &mut AkashicState)>,
    task_maanger: ResMut<TaskManager>,
) {
    for (entity, _, mut akashic_state) in query.iter_mut() {
        match *akashic_state {
            AkashicState::RoundCompleted => {
                // todo:
                // 进行故事生成
                *akashic_state = AkashicState::NarratorCompleted;
                todo!()
            }
            _ => {
                continue;
            }
        }
    }
}
