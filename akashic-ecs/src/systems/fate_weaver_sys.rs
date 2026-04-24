use bevy_ecs::{
    entity::Entity,
    message::MessageWriter,
    system::{Query, Res, ResMut},
};

use crate::{
    components::fate_weaver::{FateFrame, FateWeaver, write_shared_fate_context},
    resources::{
        task_manager::{TaskKind, TaskManager, TaskStatus},
        turn_state::{TurnPhase, TurnState},
    },
    turn_messages::TurnEvent,
    utils::parse_json_response,
};

// FateWeaver 根据当前 phase 决定是否启动 Fate 任务。
pub fn fate_weaver_system(
    turn_state: Res<TurnState>,
    query: Query<(Entity, &FateWeaver)>,
    mut task_manager: ResMut<TaskManager>,
) {
    if !matches!(
        turn_state.phase,
        TurnPhase::FateWeaving | TurnPhase::FateConsequence
    ) {
        return;
    }

    let Ok((entity, fate_weaver)) = query.single() else {
        return;
    };

    if task_manager.task_status(entity).is_none() {
        task_manager.spawn_task(entity, TaskKind::FateWeaving, fate_weaver.get_context());
    }
}

// Fate 结果解释与上下文回写放在独立 apply system，不放进回合状态机。
pub fn fate_result_apply_system(
    mut fate_weaver_query: Query<(Entity, &mut FateWeaver)>,
    turn_state: Res<TurnState>,
    mut task_manager: ResMut<TaskManager>,
    mut event_writer: MessageWriter<TurnEvent>,
) {
    if !matches!(
        turn_state.phase,
        TurnPhase::FateWeaving | TurnPhase::FateConsequence
    ) {
        return;
    }

    let Ok((entity, mut fate_weaver)) = fate_weaver_query.single_mut() else {
        return;
    };

    let Some(task_result) = task_manager
        .task_result(entity)
        .filter(|task_result| task_result.kind == TaskKind::FateWeaving)
    else {
        return;
    };

    match task_result.status {
        TaskStatus::Pending | TaskStatus::Running => {}
        TaskStatus::Done => {
            let raw_output = task_result
                .result
                .and_then(Result::ok)
                .unwrap_or_else(|| task_result.chunks.join(""));

            let frame = match parse_json_response::<FateFrame>(&raw_output) {
                Ok(frame) => frame,
                Err(message) => {
                    event_writer.write(TurnEvent::TaskFailed {
                        turn_id: turn_state.active_turn_id,
                        stage: turn_state.phase,
                        entity,
                        message,
                    });
                    task_manager.clear_task(entity);
                    return;
                }
            };

            let fate_summary = frame.summary();

            fate_weaver.push_frame(frame.clone());
            write_shared_fate_context(fate_weaver.get_context_mut(), &fate_summary);

            match turn_state.phase {
                TurnPhase::FateWeaving => {
                    event_writer.write(TurnEvent::SceneFateGenerated {
                        turn_id: turn_state.active_turn_id,
                        scene_facts: fate_summary,
                    });
                }
                TurnPhase::FateConsequence => {
                    event_writer.write(TurnEvent::ConsequenceFateGenerated {
                        turn_id: turn_state.active_turn_id,
                        consequence_facts: fate_summary,
                    });
                }
                _ => {}
            }
            task_manager.clear_task(entity);
        }
        TaskStatus::Error => {
            let message = task_result
                .result
                .and_then(Result::err)
                .unwrap_or_else(|| "fate task failed".to_string());

            event_writer.write(TurnEvent::TaskFailed {
                turn_id: turn_state.active_turn_id,
                stage: turn_state.phase,
                entity,
                message,
            });
            task_manager.clear_task(entity);
        }
    }
}
