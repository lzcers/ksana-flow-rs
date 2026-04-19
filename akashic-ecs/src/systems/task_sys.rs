use bevy_ecs::{
    entity::Entity,
    system::{Query, ResMut},
};

use crate::{
    components::AkashicState,
    resources::task_manager::{TaskManager, TaskStatus},
};

//  轮询所有在推演中的任务，推动其状态到 FateWeavingCompleted
pub fn task_system(
    mut task_manager: ResMut<TaskManager>,
    mut query: Query<(Entity, &mut AkashicState)>,
) {
    for (entity, mut akashic_state) in query.iter_mut() {
        match *akashic_state {
            AkashicState::FateWaeving => {
                // poll 任务，当其完成时更新状态
                match task_manager.poll_task(entity) {
                    TaskStatus::Done => {
                        *akashic_state = AkashicState::FateWeavingCompleted;
                    }
                    _ => {
                        continue;
                    }
                }
                todo!()
            }
            _ => {
                continue;
            }
        }
    }
}
