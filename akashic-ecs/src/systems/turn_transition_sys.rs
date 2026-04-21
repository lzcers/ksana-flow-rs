use bevy_ecs::{
    entity::Entity,
    query::With,
    system::{Query, ResMut},
};

use crate::{
    components::{
        fate_weaver::FateWeaver, protagonist::Protagonist, upper_narrator::UpperNarrator,
    },
    resources::{
        task_manager::{TaskKind, TaskManager, TaskStatus},
        turn_state::{TurnPhase, TurnState},
    },
};

// 统一解释任务完成快照，并推进故事回合 phase。
pub fn turn_transition_system(
    fate_weaver_query: Query<Entity, With<FateWeaver>>,
    protagonist_query: Query<Entity, With<Protagonist>>,
    upper_narrator_query: Query<Entity, With<UpperNarrator>>,
    mut task_manager: ResMut<TaskManager>,
    mut turn_state: ResMut<TurnState>,
) {
    match turn_state.phase {
        TurnPhase::Idle => {
            let Ok(entity) = fate_weaver_query.single() else {
                return;
            };

            if let Some(snapshot) = task_manager.task_snapshot(entity) {
                if snapshot.kind == TaskKind::FateWeaving {
                    turn_state.phase = TurnPhase::AwaitingFateResult;
                }
            }
        }
        TurnPhase::AwaitingFateResult => {
            let Ok(entity) = fate_weaver_query.single() else {
                return;
            };

            match task_manager.task_snapshot(entity) {
                Some(snapshot) if snapshot.kind == TaskKind::FateWeaving => match snapshot.status {
                    TaskStatus::Pending | TaskStatus::Running => {}
                    TaskStatus::Done => {
                        // TODO: 在这里读取 Fate 任务结果，回写 FateLine 与上下文。
                        // TODO: 根据是否存在 choices，决定进入 AwaitingProtagonist 或 AwaitingNarration。
                        task_manager.clear_task(entity);
                        turn_state.phase = TurnPhase::AwaitingProtagonist;
                    }
                    TaskStatus::Error => {
                        // TODO: 补充 Fate 任务失败态与错误恢复策略。
                        task_manager.clear_task(entity);
                        turn_state.phase = TurnPhase::Idle;
                    }
                },
                _ => {}
            }
        }
        TurnPhase::AwaitingProtagonist => {
            let Ok(entity) = protagonist_query.single() else {
                return;
            };

            match task_manager.task_snapshot(entity) {
                Some(snapshot) if snapshot.kind == TaskKind::ProtagonistAction => {
                    match snapshot.status {
                        TaskStatus::Pending | TaskStatus::Running => {}
                        TaskStatus::Done => {
                            // TODO: 在这里读取主角任务结果，并把结果回写到共享上下文。
                            task_manager.clear_task(entity);
                            turn_state.phase = TurnPhase::AwaitingNarration;
                        }
                        TaskStatus::Error => {
                            // TODO: 补充主角任务失败态与恢复策略。
                            task_manager.clear_task(entity);
                            turn_state.phase = TurnPhase::Idle;
                        }
                    }
                }
                _ => {}
            }
        }
        TurnPhase::AwaitingNarration => {
            let Ok(entity) = upper_narrator_query.single() else {
                return;
            };

            match task_manager.task_snapshot(entity) {
                Some(snapshot) if snapshot.kind == TaskKind::Narration => match snapshot.status {
                    TaskStatus::Pending | TaskStatus::Running => {}
                    TaskStatus::Done => {
                        // TODO: 在这里读取叙事任务结果，并把结果回写到共享上下文或事件流。
                        task_manager.clear_task(entity);
                        turn_state.phase = TurnPhase::Idle;
                    }
                    TaskStatus::Error => {
                        // TODO: 补充叙事任务失败态与恢复策略。
                        task_manager.clear_task(entity);
                        turn_state.phase = TurnPhase::Idle;
                    }
                },
                _ => {}
            }
        }
        TurnPhase::RoundCompleted => {}
    }
}
