use bevy_ecs::{
    entity::Entity,
    system::{Query, ResMut},
};

use crate::{
    components::{AkashicState, protagonist::Protagonist},
    resources::task_manager::{TaskManager, TaskStatus},
};

pub fn protagonist_system(
    mut query: Query<(Entity, &Protagonist, &mut AkashicState)>,
    task_manager: ResMut<TaskManager>,
) {
    for (entity, _, mut akashic_state) in query.iter_mut() {
        match *akashic_state {
            // 处理与用户交互的状态
            AkashicState::AwaitingProtagonist => {
                //  生成主角行动后
                *akashic_state = AkashicState::ProtagonistCompleted;
                todo!()
            }
            _ => {
                continue;
            }
        }
    }
}
