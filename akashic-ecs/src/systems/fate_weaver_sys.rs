use bevy_ecs::{
    entity::Entity,
    system::{Query, ResMut},
};

use crate::{
    components::fate_weaver::{FateLine, FateWeaver},
    resources::task_manager::{TaskKind, TaskManager, TaskStatus},
    resources::turn_state::{TurnPhase, TurnState},
};

// 合并处理 FateWeaver 的任务发起与结果应用骨架。
pub fn fate_weaver_system(
    mut query: Query<(Entity, &FateWeaver, &mut FateLine)>,
    mut task_manager: ResMut<TaskManager>,
    mut turn_state: ResMut<TurnState>,
) {
    let Ok((entity, fate_weaver, mut _fate_line)) = query.single_mut() else {
        return;
    };

    match turn_state.phase {
        TurnPhase::Idle => {
            task_manager.spawn_task(entity, TaskKind::FateWeaving, fate_weaver.get_context());
            turn_state.active_fate_weaver = Some(entity);
            turn_state.phase = TurnPhase::AwaitingFateResult;
        }
        TurnPhase::AwaitingFateResult => {
            if turn_state.active_fate_weaver != Some(entity) {
                return;
            }

            match task_manager.task_status(entity) {
                Some(TaskStatus::Pending | TaskStatus::Running) => {}
                Some(TaskStatus::Done) => {
                    // TODO: 在这里读取 Fate 任务结果，回写 FateLine 与上下文。
                    // TODO: 根据是否存在 choices，决定进入 AwaitingProtagonist 或 AwaitingNarration。
                    task_manager.clear_task(entity);
                    turn_state.active_fate_weaver = None;
                    turn_state.phase = TurnPhase::AwaitingProtagonist;
                }
                Some(TaskStatus::Error) => {
                    // TODO: 补充失败态与错误恢复策略。
                    task_manager.clear_task(entity);
                    turn_state.active_fate_weaver = None;
                    turn_state.phase = TurnPhase::Idle;
                }
                None => {}
            }
        }
        _ => {}
    }
}
