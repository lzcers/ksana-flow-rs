use bevy_ecs::{
    entity::Entity,
    system::{Query, ResMut},
};

use crate::{
    components::{
        AkashicState,
        fate_weaver::{FateLine, FateWeaver},
    },
    resources::task_manager::{TaskManager, TaskStatus},
};

// 推动故事演进
pub fn fate_weaving_system(
    mut query: Query<(Entity, &FateWeaver, &mut FateLine, &mut AkashicState)>,
    mut task_manager: ResMut<TaskManager>,
) {
    for (entity, fate_weaver, fate_line, mut akashic_state) in query.iter_mut() {
        match *akashic_state {
            AkashicState::Idle => {
                // todo: 开始推演剧情
                task_manager.spawn_task(entity, fate_weaver.get_context());
                *akashic_state = AkashicState::FateWaeving;
                todo!()
            }
            AkashicState::FateWeavingCompleted => {
                // todo:
                // 根据结果，如果有 choices 则进入等待主角行动状态
                // 否则直接进入本轮完成阶段
                // if choices != empty
                *akashic_state = AkashicState::AwaitingProtagonist;
                // else 直接本轮结束
                *akashic_state = AkashicState::RoundCompleted;
            }
            AkashicState::ProtagonistCompleted => {
                *akashic_state = AkashicState::RoundCompleted;
                todo!()
            }
            AkashicState::NarratorCompleted => {
                // 更新相关轮次状态
                *akashic_state = AkashicState::Idle;
                continue;
            }
            _ => {
                todo!()
            }
        }
    }
}
